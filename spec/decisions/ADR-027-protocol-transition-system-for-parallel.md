---
id: ADR-027
title: "The protocol transition system for parallel"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-170
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-171
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-177
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-228
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-425
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-426
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-427
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-428
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-429
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-430
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-431
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-432
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-433
    type: depends_on
---
# ADR-027: The protocol transition system for parallel

## Status

Proposed, 2026-10-01. §10 records the owner's rulings on the five questions
the draft left open. The QSL compiler requirements that implement it are
FR-205 to FR-218, traced to US-024; QSpec's half is listed under References.
The owner ruled that this record comes before the refinement, state-space
reduction and weak-memory records, which build on it. The owning ticket and
those records are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `PS-`, `FO-`, `ST-`, `AT-`, `CI-`, `SE-`, `PB-`, `PD-`,
`PA-`, `TS-`, `PX-`, `FT-`, `PR-`, `QS-` and `DS-` are local to this record.
Other artifacts cite them as `ADR-027 ST-6`. Items of ADR-018 are cited as
`ADR-018 SM-3`.

## Context

**What `parallel` means today.** QSpec FR-052: "Parallel branches SHALL admit
every interleaving that preserves each branch's causal edges. A join SHALL
name the branches and completion rule it awaits." QSpec FR-228 lists
`parallel-branch` and `join` among the admitted control constructs, and its
complete control contract gives the join policies `all`, `any`, `quorum(n)`
and `predicate(f)`, each declaring whether outstanding branches continue or
are cancelled, with cancellation an explicit control transition. QSpec
FR-171 states when each policy settles: `all` after every branch settles,
`any` after the first, `quorum(n)` after `n` distinct branches, a predicate
join when its pure predicate is true over the branch-result map. The shared
grammar spells it `parallel P { branch a … branch b … } join all [a, b];`
with the complete form `join <policy> [ … ] [outstanding continue|cancel]`.

The compiled protocol form (docs/compiled-protocol-v1.md) is
`{kind:"parallel", branches, join}` inside a control tree of `sequence`,
`choice`, `repeat`, `await`, event nodes (`send`, `receive`, `attempt`,
`effect`, domain `event`), `check` and `commit`, with a `finish` node. Its
structural edges are in → each branch and every branch → out; a `receive`
adds send.out → receive.in and an `effect` adds attempt.out → effect.in.
Choice and repeat guards are Boolean formulas over observed fields of event
binders, each read at its original observation anchor (QSpec FR-053's
visible fragment). An `attempt` invokes an operation of a model. QSpec
FR-170 gives channels a queue transition system: `send`, `deliver`,
`duplicate` and `lose`, with capacity and overflow.

**What explores it.** Nothing. FR-120's `ModelSystem` is QSL's only
`TransitionSystem` (ADR-016 FP-4). Its successor relation interleaves every
operation on every receiver; it has no control state. S3 checks a protocol's
`sequence`, `attempt` and `finish` nodes and refuses the other kinds with
`unsupported_construct`/`not-yet-implemented` (ADR-017 PF-7). S4 emits no
protocol node.

**What ADR-018 gives.** A model subject (SM-2), behaviours as maximal paths
(SM-3), terminal stutter (SM-4), weak fairness over operations (FA-1 to
FA-6), deadlocks as a derived item (DL-1 to DL-7), the explicit-state product
EN-1 over FR-101's engine, and counterexamples that replay through
`ModelSystem` (CX-1 to CX-5). QSpec FR-181's state key already has members
for `control`, `queues`, `roles`, `observations` and `bounds`, all empty for
`ModelSystem`.

**What the dependent records assume.** The refinement record (ADR-020) maps
each concrete operation to an abstract step and treats QSpec FR-177's
protocol refinement as the same check with observations as step labels, once
a protocol `TransitionSystem` exists (its MC-1). The state-space reduction
record (ADR-021) takes static read and write footprints per transition
identity (its POR-1) and names "transitions of distinct `parallel` branches
as the source of independence" (its POR-9). The weak-memory record (ADR-025)
reads each attempt as one indivisible step under `sc`, adds a memory
component to the state, internal memory steps such as `flush(b)`, and
enabledness conditions on fork, join and `send` (its SC-1, MS-1, TSO-7).
Each of those needs one definition of the protocol state, its steps and their
identities.

## Decision

### 1. Protocol subjects and protocol states

| ID | Rule |
| --- | --- |
| PS-1 | **Protocol subject.** A protocol subject is a model subject (ADR-018 SM-2) together with one checked protocol clause of its package, a binding of the clause's `over` parameter to a value of its declared type, a binding of each static role, and the universe of each replicated role's population (RR-1). The protocol's steps are the only steps: the model's operations run only as the protocol's attempts. A clause with activation `on origin` runs one **protocol instance** from each initial state; a clause with `activation on each` starts one instance per trigger occurrence (AE-1). The protocol clause, the bindings, the protocol's `terminal` and scheduling members (PD-1, PA-3) and the resolved memory model of each outermost `parallel` (SE-4) are source or subject, and so enter the obligation identity (ADR-018 §1 "Scope"). |
| PS-2 | **Threads.** Each protocol instance's `run` control is its **root thread**, with path `(instance ordinal)` (AE-2). Each fork of a `parallel` (ST-6) creates one **branch thread** per branch, and each activated compensation registration one **compensation thread** (CP-3). A branch thread's **path** is its parent thread's path followed by `(parallel node, branch label, ordinal)`; a compensation thread's is its instance's path followed by `(compensation template, ordinal)`. Each ordinal is the smallest natural number that no live sibling of the same node holds, so a `parallel` inside a `repeat` whose earlier instance still runs (`outstanding continue`) gets a second instance with its own paths. `parallel` controls nest to any depth, and thread, instance and registration counts have no ceiling of their own; EN-1's limits (ADR-018 §3) and the caller's bounds (AE-4) are the only bounds on a run. |
| PS-3 | **Control positions.** Each live thread has one **rest point** (FO-1): an event node, a `parallel` waiting to fork, the join of a `parallel` it forked, an `await`, a `check` whose value is false, the `finish` node (root only), a compensation attempt or a compensation retry point (compensation threads, CP-3), or **end**. Each live thread also has a **disposition**: `running`, `completed` or `cancelled`. A thread is live while it is `running`, or while its parallel instance's join has not yet read it. |
| PS-4 | **Parallel instances.** Each fork creates one parallel instance, keyed by the parent thread's path, the `parallel` node and the ordinal, holding its branch threads' dispositions and a **settled** flag. The instance is removed once its join has settled and every branch thread has left `running`. |
| PS-5 | **Binders.** An event step binds its binder to the event's record (ST-1 to ST-4, ST-9). A binder value is immutable. It is kept while some rest point of some live thread can still reach a node that reads it, by S3's scope rules (a branch binder stays available after an all-branch join, as the compiled form states); it is removed in the step that makes it unreachable. |
| PS-6 | **Iteration counters.** A `repeat` needs progress: every cycle in its body graph passes through a step node, so settling (FO-1) always reaches a rest point. The **body graph** of a `repeat` is the control-flow graph of its body with the edge from the body's end back to the guard: a node for each control in the body, nested controls included, an edge for each structural move FO-1 can take between them (into and out of a `sequence`, into each `choice` case, into a nested `repeat`'s body, back to its guard and out past it, whatever that guard's value), and the back edge. A **step node** is a node whose control takes a step before the thread moves on: an event node (`attempt`, `effect`, domain `event`, `commit`, `send`, `receive`), an `await` (its match or its `timeout` step), a `parallel` (its `fork` and `join` steps) and `finish`. A body whose `choice` has one case with a step and another with none, or whose nested `repeat` can be skipped with no step elsewhere on the cycle, has no progress. Its iteration maximum and its invariant and variant are optional, and only a termination claim reads the variant (QSpec FR-052). A `repeat` with an authored maximum keeps one counter per (repeat node, thread path) while the thread is inside it, from 0 to the maximum. A `repeat` with no maximum keeps none. |
| PS-7 | **Queues.** Each channel has one queue per protocol instance: a sequence in delivery order for `fifo` ordering, per FIFO key, and a bag for `unordered`. An entry is the send node, the sending thread's path and the payload. The queue's size is bounded only by the channel's authored capacity; a channel with none has an unbounded queue. |
| PS-8 | **Protocol state and key.** A protocol state is (model state, protocol instances, rest points and dispositions, parallel instances, compensation registrations, binders, iteration counters, queues, role instances, memory component). Its key is QSpec FR-181's simulator state key: `semantic` is FR-120's population map; `control` maps each live thread's path to its rest point and disposition, each parallel instance's key to its dispositions and settled flag, and each compensation registration's key (CP-1) to its status and captures; `observations` maps (binder node, thread path) to the binder value; `bounds` maps (repeat node, thread path) to the counter and each compensation thread's path to its attempt count; `queues` maps each (protocol instance ordinal, channel, FIFO key) to its contents; `roles` maps each (protocol instance ordinal, role) to a static role's binding or a replicated role's instances' objects with their status (RR-3), as QSpec FR-425 keys them. Each map is sorted by JCS bytes, as FR-181 states. The memory component is a member the memory model supplies (SE-1 a), empty under `sc`. Two states with equal keys are one state. The ordinal rules (PS-2, AE-2), PS-4's and PS-5's removal and AE-3's removal make the key a function of what the protocol can still do, not of how it got there. |

### 2. Steps

#### 2.1 Folding structural moves

| ID | Rule |
| --- | --- |
| FO-1 | **Settling.** After every step, each thread whose rest point the step moved, and each thread the step created, is **settled**: its structural moves are applied until it reaches a rest point (PS-3). The moves are: enter a `sequence` at its first child, or leave it when empty; leave a child for its successor, and leave the last child for the parent's continuation; at a `choice`, enter the one case whose guard is true; at a `repeat`, enter the body when the guard is true and, for a `repeat` with a maximum, the counter is below it, enter `exhausted` when the guard is true at the maximum, leave when the guard is false, and advance a counter on each re-entry (the compiled form's `repeat_progress` edge); at a `check`, continue when its value is true and rest there when it is false; at the end of a branch body, set the disposition `completed` and rest at **end**; at the end of `run`, rest at `finish`. Three effects are folded into the step that causes them, not taken as steps of their own: a compensation registration into its forward effect's step (CP-1), a registration's activation into the step that binds its first eligible trigger (CP-2), and a replicated role instance's `workflow` or `scope` retirement into the step that ends its lifetime (RR-4). |
| FO-2 | **Folding is deterministic.** Choice guards, repeat guards and checks read binders and constants only, each atom at its binder's observation anchor (QSpec FR-053's visible fragment). Binder values are immutable (PS-5), so a guard's value does not depend on when its thread reaches it, and settling is a function of the state. QSpec FR-053 makes choice guards exhaustive and exclusive, so exactly one case is entered. A structural move is therefore never a step and never a position. |
| FO-3 | **Initial states.** For each initial model state, the initial protocol state holds it, the root thread settled from the start of `run`, empty queues, no binders and the role bindings. |

#### 2.2 Step kinds

Each row is one step kind. "At" means the owning thread rests at that node.
Every step ends with FO-1. "Gate" is the memory model's enabledness condition
(SE-1 e), true under `sc`. An event step `by r` for a replicated role `r`
(ST-1 to ST-4) is taken by one live instance of `r`, named in its transition
identity, with one step per live instance (RR-2).

| ID | Step | Owner | Enabled when | Effect |
| --- | --- | --- | --- | --- |
| ST-1 | **`attempt(n)`** | the thread at attempt node `n` | For one FR-120 application of `n`'s operation (receiver and argument vector from the finite domains), the effective precondition over the contract clauses `n` selects (its checked `contracts`) is true over the thread's observation (SE-1 b); some post-state within the operation's frame satisfies the selected postconditions; the binder's record (receiver, arguments, result) satisfies `n`'s constraint; and the gate holds | The post-state's delta is written through the memory model (SE-1 c), in place under `sc`; the binder is bound to the record. One step per (application, post-state, result), as FR-120 gives one successor per post-state |
| ST-2 | **`event(n)`** for an `effect`, domain `event` or `commit` node | the thread at `n` | Some record of `n`'s record type satisfies `n`'s constraint; for an `effect`, its attempt's binder is bound | The binder is bound. The model state is unchanged. One step per satisfying record |
| ST-3 | **`send(n)`** | the thread at send node `n` | Some payload of the channel's message type satisfies `n`'s constraint; the channel's capacity and overflow policy admit the send (QSpec FR-170: under `block`, not while the queue is full); the gate holds | Per QSpec FR-170: append the entry; under `reject` at full capacity, leave the queue unchanged and record the send as rejected in its binder; under `loss`, apply the declared loss selector. The binder is bound |
| ST-4 | **`receive(n)`** | the thread at receive node `n` | The queue holds a deliverable entry from the send node `n` names (`fifo`: the head of its FIFO key; `unordered`: any entry) whose payload satisfies `n`'s constraint; the gate holds | The entry is removed and the binder bound to its payload. One step per distinct deliverable entry |
| ST-5 | **`duplicate(h, e)`** and **`lose(h, e)`** | channel `h` | The channel's delivery policy admits duplication (`unknown`, `at-least-once`) or loss (`unknown`, `at-most-once`), and entry `e` is in the queue; for `duplicate`, the capacity admits one more entry | Per QSpec FR-170: add a copy of `e`, or remove `e` |
| ST-6 | **`fork(p)`** | the thread at `parallel` `p` | The gate holds | Creates the parallel instance (PS-4) and one branch thread per branch, each `running` and settled from its branch body's start; the owner rests at `p`'s join. The memory model splits its component (SE-1 f) |
| ST-7 | **`join(p)`** | the thread at `p`'s join | The join policy is satisfied by the instance's dispositions, a thread counting once it is `completed` (QSpec FR-171): `all`, every branch; `any`, one; `quorum(n)`, `n`; `predicate(f)`, `f` true over the disposition map; and the gate holds | Sets the instance settled. Under `outstanding cancel`, every branch thread still `running` becomes `cancelled` in the same step and takes no further step. Under `outstanding continue`, those threads keep running. The owner settles past the join. The memory model merges its component (SE-1 f) |
| ST-8 | **`timeout(a)`** | the thread at `await` `a` | Always | The owner settles into `a`'s `timeout` control. The await's match is its event node's own step (ST-2 to ST-4), after which the owner settles into `then` |
| ST-9 | **`finish`** | the root thread at `finish` | Every other thread of the instance, compensation threads included, has left `running`, and some record of the finish type satisfies the finish constraint | The binder is bound and the protocol instance is **finished**: it is removed in this step (AE-3) |
| ST-10 | **Memory step** | the memory model | SE-1 d | SE-1 d |
| ST-11 | **`cattempt(c, r)`** | the compensation thread of registration `r` of template `c` | The thread rests at its compensation attempt or retry point; its attempt count is below the template's authored `attempts n`; no step of the commit node the template names (`commit Node`) has occurred in its instance; for a retry, the template's retry relation holds over (the previous attempt record, this one); and ST-1's conditions hold for an application of the template's operation by its role, with the template's attempt type as the record | As ST-1. The attempt count advances; the thread rests at its retry point |
| ST-12 | **`cend(c, r)`** | the compensation thread at its retry point | Always | The thread ends and is removed. The template's recovery predicate is evaluated over the thread's observation, the registration and activation captures and the recovery view the model state gives; the registration's status becomes `recovered` when it is true and `unrecovered` otherwise |
| ST-13 | **`activate(T)`** | the environment of an `on each` protocol | Some record of the trigger type `T` satisfies the activation guard. At the instance bound the step stays enabled and is not expanded (AE-4) | Starts one protocol instance (AE-2) with its captures evaluated over the trigger record, its root thread settled from the start of `run` |
| ST-14 | **`spawn(r, o)`** | the protocol instance | `o` is an object of replicated role `r`'s population in the subject's universe, not yet an instance of `r` in this protocol instance; fewer than the role's authored `max` instances of `r` are live | `o` becomes a live instance of `r` (RR-2) |
| ST-15 | **`retire(r, o)`** | the protocol instance | `r`'s lifetime is `until(p)` and `p` holds for instance `o` over the binders in scope and the model observation | Instance `o` of `r` is retired: it takes no further step and is never spawned again in this protocol instance |

**Waiting.** A thread at an attempt whose application is not
enabled, at a receive with no deliverable entry, at a join whose policy is
not yet satisfied, or at a false `check`, has no enabled step and waits. A
`check` whose value is false never becomes true (FO-2), so its thread waits
for ever, and the state reports it when nothing else can move (PD-2). The
protocol system reads no clock: `timeout(a)` is enabled for as long as `a`
waits, which admits every behaviour that some deadline admits. The same
holds for a compensation's activation window `within [a, b]`, which is not
read.

#### 2.2a Protocol instances, replicated roles and compensation

| ID | Rule |
| --- | --- |
| AE-1 | **Activation on each.** A clause with `activation on each (x: T) when {g}` starts one protocol instance per `activate(T)` step (ST-13). The trigger is a semantic event the environment supplies; the protocol system does not derive it from the model's own steps. Each instance has its own root thread, captures, role bindings, replicated role instances, registrations, binders and queues, and every instance shares the one model state. Under `on origin` there is one instance, ordinal 0, started at position 0 (FO-3). |
| AE-2 | **Instance identity.** An instance's ordinal is the smallest natural number no live instance holds. Its threads' paths begin with it (PS-2). |
| AE-3 | **Instance removal.** Under every activation kind, an instance is removed from the state in the step that finishes it (ST-9); its binders, registrations, role instances and queues go with it. A trigger record is not kept, so two instances started by equal records are not told apart once both are removed. |
| AE-4 | **Instance bound.** `max_live_instances` is a method bound of EN-1, as `max_depth` is a search horizon (ADR-018 §1) and the weak-memory record's memory bounds are: a bound on the explored state space that every result states, not a resource limit. The request sets it, with no ceiling, and a request that leaves it unset gets the published default of 3. An `activate` step from a state with that many live instances is not expanded; the state is **instance-limited**, FA-2 enabledness reads `activate` as enabled, and the state is never terminal or a deadlock. A run that completes with no counterexample and at least one instance-limited state settles V-6, `inconclusive`, cause `InstanceBoundReached{bound, states}`, with `bound` naming the request member that raises it, `max_live_instances`, and its value, and `states` the count of instance-limited states; a counterexample found is real and settles V-4; a run that completes with no counterexample and no instance-limited state explored every instance count the protocol reaches and settles V-1. Precedence among the under-approximation causes is the reduction record's `ConstraintReached`, then the weak-memory record's `MemoryBoundReached`, then `InstanceBoundReached`, then `BoundReached` (the completed `max_depth` search horizon). Every verdict over an `on each` protocol, V-1 included, records in its FR-331 terminal record the `max_live_instances` value used and whether it was reached. The bound is a method parameter, not part of the subject or its obligation identity. |
| RR-1 | **Replicated roles.** A role `role r each T from pop max n lifetime L` has one **role instance** per object of population `pop` it spawns. The population's universe is the subject's, set by the caller (ADR-018 SM-2), so the caller sets how many objects can ever be instances. `max n` is the protocol's own authored rule on how many instances are live at once (QSpec shared grammar), part of the checked package; QSL adds no bound of its own. |
| RR-2 | **Spawn and action.** `spawn(r, o)` (ST-14) makes object `o` an instance. An event node `by r` is taken by any live instance of `r`, one step per instance, and its binder records the acting instance. The instance identity in the state key is the object's reference (QSpec FR-171's ordinal is trace data, the step at which the spawn occurred, recomputed by replay), so equal instance sets key equal whatever order they were spawned in. |
| RR-3 | **Key.** The `roles` member maps each replicated role to its instances, each object reference with status `live` or `retired`. A retired instance stays in the key so that it is never spawned again, as QSpec FR-171-AC-5 requires; the key is finite because the universe is. |
| RR-4 | **Lifetime.** `workflow`: the instance retires when its protocol instance finishes (AE-3). `scope`: it retires in the step after which no live thread can reach any node `by r` inside the smallest control that holds every such node, which S3 computes. `until(p)`: `retire(r, o)` (ST-15). A retired instance takes no step. |
| CP-1 | **Registration.** When the step of a template's forward `effect` node occurs (ST-2), the same step registers one compensation for each template naming that node, keyed by (template, the effect's thread path, ordinal), the ordinal the smallest no live registration of the same key prefix holds. The template's `as` binder is bound to the effect record and its registration captures are evaluated then. A registration is `registered` until it activates or closes. No registration is made after a step of the template's commit node has occurred in the instance (QSpec FR-057). |
| CP-2 | **Activation.** A `registered` registration activates in the first later step that binds a record of the template's trigger type for which the activation guard holds over that record and the registration captures. Activation is folded into that step (FO-1): the activation captures are evaluated, the registration becomes `activated`, and its compensation thread is created. Later eligible records do not reactivate it or register another. Two eligible triggers in sibling branches activate it in the order the behaviour takes them; both orders are behaviours (CI-3). |
| CP-3 | **Compensation thread.** An activated registration's thread starts at its compensation attempt with attempt count 0, takes `cattempt` (ST-11) at most `attempts n` times, the template's authored count, and rests at its retry point after each. Each attempt is an ST-1 application of the template's operation by its role, so it has FR-120's meaning and the same atomicity (AT-2). |
| CP-4 | **Commit.** After a step of the template's commit node (`commit Node`) occurs in the instance, its registrations that are still `registered` close, and its compensation threads can take no further `cattempt`; each can still take `cend`. `commit never` closes nothing. |
| CP-5 | **Ending.** At its retry point a compensation thread can retry, when ST-11 is enabled, or end with `cend` (ST-12), which is always enabled: the retry relation constrains a retry and never forces one (QSpec FR-058). `cend` records the recovery predicate's value as the registration's status. A finished instance needs every compensation thread ended (PB-4); a registration that never activates does not hold up finishing and closes when its instance finishes. |

#### 2.3 Atomicity

| ID | Rule |
| --- | --- |
| AT-1 | **Every step is atomic.** One step evaluates its enabledness, applies its effect and settles its threads, and no step of another thread or of the memory model runs inside it. |
| AT-2 | **An attempt is one step.** The precondition, the choice of post-state and result, the frame check, the binder constraint and the write happen together, at the step's position. This is FR-120's successor rule applied to one application, the same rule `ModelSystem` applies, so an attempt is sequentially consistent at the granularity of a whole operation. A concurrent implementation inherits a protocol proof when each bound function's concurrent effect is as indivisible as its attempt (ADR-017 AR-2); the weak-memory record states the conditions under which a single-location attempt is split into memory steps (SE-1). |
| AT-3 | **No pending attempt.** A thread is never inside an attempt between two positions. A thread whose rest point is an attempt that is not enabled is waiting at it (§2.2); that is the only sense in which an attempt is pending. |

#### 2.4 Causal order and interleavings

| ID | Rule |
| --- | --- |
| CI-1 | **Every causal edge is enforced.** A step of node `m` is enabled only when every causal predecessor of `m` in the compiled edge relation has occurred: a `sequence` edge by the owning thread's rest point; a `branch` edge by the fork that created the thread; a `join` edge by ST-7; send.out → receive.in by the queue (ST-4 reads an entry only from `m`'s send node); attempt.out → effect.in by ST-2's binder condition; an await's anchor edge by the anchor's step, which precedes the await in its thread. |
| CI-2 | **No other order is added.** A step waits on a step of another thread only through a join, a queue, a binder, a memory gate (SE-1 e) or the model state its own precondition or constraint reads. Source order, array order, timestamps and FIFO order on another channel add nothing, as QSpec FR-052-AC-2 requires. Canonical transition order (FR-181) orders the search and the choice of counterexample, never the set of behaviours. |
| CI-3 | **The causal interleavings are the behaviours.** Project a behaviour onto its event steps (ST-1 to ST-4, ST-9 and ST-11). The projections are exactly the linear extensions of the causal edge relation, unfolded along the choices and iterations the behaviour takes, in which each event step is enabled when it is reached. This is QSpec FR-052's "every interleaving that preserves each branch's causal edges" (its causal-interleavings rules, tested by QSpec FR-052-AC-6), with an attempt's precondition and an event's constraint as the only further conditions. For a protocol whose preconditions and constraints are all true and that uses no channel, the projections are every linear extension. |

#### 2.5 The memory-model seam

| ID | Rule |
| --- | --- |
| SE-1 | **The seam.** The protocol system reaches memory through one interface, `MemoryModel`, with eight members: (a) **component**, the memory state, a member of the state key; (b) **observe**, the observation of the model state that a thread's attempt evaluates its contracts over; (c) **write**, where an attempt's post-state delta goes; (d) **internal steps**, each with a typed canonical transition identity, an enabledness condition and an effect, and no operation; (e) **gate**, an extra enabledness condition on `fork`, `join`, `send`, `receive` and `attempt` steps of a thread; (f) **split and merge**, what a `fork` gives each branch thread and what a `join` gives the parent; (g) **atom values**, the value of each location that a temporal atom reads; (h) **fairness**, constraints the model adds to every infinite-trace clause, with a `memory` origin. |
| SE-2 | **`sc`.** The sequentially consistent model is the seam's identity: an empty component; observe and atom values read the model state; write applies in place; no internal steps; every gate true; split and merge do nothing; no fairness. Every rule of this record that names the seam holds under `sc` with these values. |
| SE-3 | **Memory steps are steps.** A memory step is a position (PB-2) that does not count for step-counted intervals (PB-3, QSpec FR-431), has the footprint its memory model gives it (FT-1), and is enabled and applied by its model's rules alone. It is owned by no thread and moves no rest point. |
| SE-4 | **Selection.** The memory model a `parallel` runs under is its **resolved model**: the request's selection for its outermost `parallel` when present, else the source default (the weak-memory record's MM-2). The resolved model of each outermost `parallel` is part of the subject and its obligation identity; the source clause enters the identity only through it, so a source `tso` and a source `sc` with a request `tso` carry one identity. The weak-memory record defines its syntax, its models and their components, steps, gates and fairness. |

### 3. Fit with ADR-018

#### 3.1 Behaviours, positions and terminal states

| ID | Rule |
| --- | --- |
| PB-1 | **Behaviours.** A behaviour of a protocol subject is a maximal path of the protocol system from an initial protocol state (FO-3). This is ADR-018 SM-3 with the protocol system's successor relation in place of FR-120's. |
| PB-2 | **Positions and anchors.** Position 0 is the initial state, observed with an `initialization` anchor. Position `i > 0` is the post-state of the `i`-th step. An `attempt` or `cattempt` step is observed with its operation's anchor, as SM-3 states for a model step. Every other step is observed with a `protocol` anchor naming the step kind and its node (or channel, template or role), and a memory step with the anchor its model gives it. A `holds` atom evaluates over the model observation at the position, with each location read through the memory model's atom values (SE-1 g). Every step is a position for atoms and for unbounded operators. Rest points, dispositions, binders, registrations, role instances and queues are read by replay and counterexample rendering, not by atoms. |
| PB-3 | **Only operations, events, sends and receives count for intervals.** A step is **counted** when it is an operation (`attempt`, `cattempt`), an event (`event` for an effect, domain event or commit; `activate` for a trigger occurrence), a `send` or a `receive`. `fork`, `join`, `finish`, `timeout`, `cend`, `duplicate`, `lose`, `spawn`, `retire`, `fence` steps (the weak-memory record's) and memory steps are **uncounted** (QSpec FR-431). The terminal stutter step (ADR-018 SM-4) is counted, as over a model subject and as QSpec FR-431-AC-3 states: it is the stutter extension of a terminal state, not one of the protocol's control steps, so RU-4's list of counted protocol steps does not exclude it. An interval operator measures distance in counted steps: the distance from position `i` to position `j >= i` is the number of counted steps among steps `i + 1` to `j`, and `eventually[a, b] p` holds at `i` when `p` holds at some `j >= i` at distance in `[a, b]`; `always`, `until` and `release` with an interval, and the past operators with an interval (backwards), read distance the same way. Several positions can share one distance; an uncounted step moves the position without moving the distance. Over a model subject every step is counted, so ADR-018's reading is unchanged there. A bounded profile's horizon (ADR-018 TP-2) is in counted steps too. A loop with no counted step keeps every distance from a position before it constant, so an interval that needs a counted step after the loop is entered is not met on that lasso; the default scheduler fairness (PA-3) rules out such loops where a thread can still move. Stutter-invariant formulas (ADR-018 IV-7) read the same truth whether a step counts or not. |
| PB-4 | **Terminal states.** A protocol state with no enabled step is terminal (ADR-018 SM-4) and reads by SM-4: closed under a bounded profile and stutter-extended under infinite-trace, with the stutter step outside every fairness constraint. A protocol instance is **finished** once ST-9 has run in it, which ST-9 allows only after every other thread of the instance, compensation threads included, has left `running`, and the finishing step removes it (AE-3). Under `on origin`, the state after `finish` holds no live instance and has no enabled step, so every behaviour that finishes ends in its terminal stutter. Under `on each`, a state with no live instance is terminal only when no `activate` step is enabled. |
| PB-5 | **Undefined evaluation.** A claim over a protocol subject that evaluates undefined at a position it reads settles by ADR-018 UE-1 to UE-6: `refuted`, cause `UndefinedEvaluation{where, cause}`, with a finite prefix ending at that position. PA-4's machine closure makes the prefix one of a behaviour fair under every constraint, scheduler constraints included. PD-1's `P` read at a terminal state is read the same way. Replay is PX-2 followed by ADR-018 UE-5. |

#### 3.2 Deadlocks

| ID | Rule |
| --- | --- |
| PD-1 | **Intended terminal states.** A protocol clause declares intended terminal states with at most one `terminal` member, the protocol-level form of ADR-018 DL-1: `terminal when P`, where `P` is a Boolean state predicate over the model observation and the protocol's captures, evaluated at a terminal state through the one clause evaluator; or `terminal any`, every terminal state is intended, which is the per-protocol opt-out. A terminal state is intended when it holds no live instance (every instance has finished and been removed), or when the clause declares `terminal when P` and `P` holds there. The model's own `terminal` member is not read: it classifies the terminal states of the model's successor relation, which a protocol subject does not run. The spelling is illustrative; QSpec's shared grammar owns it. The member is part of the checked package and so of the obligation identity. |
| PD-2 | **Deadlock.** A deadlock of a protocol subject is a reachable terminal state that is not intended (PD-1): some thread waits (§2.2) and no step of any thread, channel, environment or memory model is enabled. A join waiting on a branch that waits at a disabled attempt, an empty queue or a false `check` is the common case. Under `on each`, `activate` may stay enabled for ever; there a state is also a deadlock when at least one instance is live, every step enabled at it is an `activate`, and `P` does not hold: no live instance can move until a new trigger arrives, and no trigger is promised. Enabledness is ST-1 to ST-15's, which ADR-018 FA-2 reads (PA-2). |
| PD-3 | **Reported by default, with a per-protocol opt-out.** The request writer adds one deadlock-freedom item per distinct protocol subject among a request's temporal items (ADR-018 DL-3), with `deadlocked` read by PD-2, unless the protocol clause declares `terminal any`. Verdicts, the counterexample kind `Deadlock` and DL-6's reading of claims at terminal states apply unchanged. |
| PD-4 | **Blocked threads.** A deadlock counterexample also carries, for its last state, each waiting thread's path, rest point and **wait cause**: `Precondition{clause}`, `NoPostState`, `Constraint`, `EmptyQueue{channel}`, `JoinWaiting{threads}`, `CheckFalse{node}` or `Gate`. Replay recomputes the list (PX-2); it is data for the reader, as the memory component is in the weak-memory record. |
| PD-5 | **Boundary states.** A state the state-space reduction record cuts (a state constraint's boundary), a memory bound limits or the instance bound limits (AE-4) is not terminal and never a deadlock. |

#### 3.3 Fairness

| ID | Rule |
| --- | --- |
| PA-1 | **Targets.** A fairness constraint of an infinite-trace clause over a protocol subject names one of four targets, each with ADR-018 FA-1's granularity: an **operation**, whose class is every `attempt` and `cattempt` step that applies it (`whole`), or one class per FR-120 transition identity (operation, receiver, arguments) of those steps (`each`), which is ADR-018 FA-1's meaning; an **attempt node**, whose class is every step of that node (`whole`), or one class per (thread path, application) (`each`); a **branch** of a `parallel`, or the root by the protocol's name, whose class is every step that its threads own, a thread owning its event steps, the forks it performs and the joins it waits at (`whole`), or one class per thread (`each`); a **compensation template**, whose class is every `cattempt` and `cend` step of its threads (`whole`), or one class per thread (`each`). A branch constraint does not cover the threads of a `parallel` nested inside it; each has its own. An unmarked constraint is `whole` (ADR-018 FA-6). The spelling is QSpec's. |
| PA-2 | **Enabled and taken.** A class is enabled at a state when one of its steps is enabled there (ADR-018 FA-2 over ST-1 to ST-15) and taken by a step that belongs to it. FA-3's weak fairness, FA-4's SCC filter and the strong-fairness kind read these classes unchanged. Channel steps (ST-5), `activate`, `spawn`, `retire` and memory steps are in no class; a memory model adds its own constraints (SE-1 h). |
| PA-3 | **Scheduler fairness is on by default.** Every infinite-trace clause over a protocol subject carries one **scheduler constraint** per thread target of the protocol: `fair weak whole` over each `parallel` branch, the root and each compensation template (PA-1). The constraints are derived from the checked protocol clause, not authored; each enters the clause's fairness set with a `scheduler` origin, so it is part of the obligation identity, every counterexample and replay, as ADR-018 FA-1 states for authored constraints and the weak-memory record for its `memory` origin. A protocol turns them off with its `scheduling adversarial` member (illustrative spelling), which is source, so the same protocol with and without it is two packages and two obligations; with it, the clause's fairness set is its authored constraints alone and a scheduler may starve any thread. Authored constraints, `each` constraints included, are added to the scheduler constraints. |
| PA-4 | **Machine closure.** Weak fairness over step classes is machine-closed, as ADR-018 §4 states for operations: every finite prefix extends to a behaviour fair under every constraint, scheduler constraints included. A fairness constraint never makes a proof vacuous and never changes a safety verdict, so the default changes liveness verdicts only. |

### 4. Fit with FR-101 and ModelSystem

| ID | Rule |
| --- | --- |
| TS-1 | **`ProtocolSystem`.** Crate `qsl-eval`'s `simulation` module gains `ProtocolSystem<M: MemoryModel>`, beside `ModelSystem`, implementing FR-101's `TransitionSystem`. It is built from the in-process checked package's checked protocol clause, the protocol subject (PS-1), the memory model, with `M = Sc` for a `parallel` with no memory clause, and the instance bound (AE-4). `initial` is FO-3, with no instance under `on each`; `key` is PS-8; `successors` enumerates ST-1 to ST-15 over every live instance, thread, registration, role, channel and the memory model, each step with its successor state. |
| TS-2 | **Transition identity.** A step's identity is a typed canonical form in QSpec FR-181's style: `{"type":"protocol-transition","step":<kind>,"thread":<path>,"node":<wire node id>,…}` with, for `attempt` and `cattempt`, the operation's qualified name, the receiver reference and the argument vector; for `cattempt` and `cend`, the template's qualified name and the registration key; for `send`, `receive`, `event`, `finish` and `activate`, the bound record; for a step `by` a replicated role, the acting instance's reference; for `spawn` and `retire`, the role and the object's reference; for `duplicate` and `lose`, the channel's qualified name with its protocol instance ordinal, and the entry; for a `fence` and an access with an ordering, the ordering; for a memory step, the form its model gives. Several successors of one identity are told apart by their post-state digest, as ADR-018 CX-3 tells FR-120's apart. Successors are visited in ascending JCS bytes of the identity, as FR-181 states. |
| TS-3 | **One application rule.** ST-1 applies one FR-120 application through the same function `ModelSystem` uses to expand an application: effective precondition, frame post-states, postcondition, `check_frame`'s delta. The protocol system passes the clauses its attempt selects and the observation the memory model gives; it adds no second contract evaluator. |
| TS-4 | **Findings.** Each expanded protocol state carries FR-120's invariant findings for its model state and `ContractUndetermined` for an attempt whose contract decision was refused or incomplete. Such a decision during a model check settles ADR-018 V-6 `UndecidedSuccessor`, as for a model subject. A contract that evaluates undefined is not undecided: the item refutes with cause `UndefinedEvaluation{where, cause}` (ADR-018 UE, QSpec FR-426). |
| TS-5 | **Pre-check.** Before any expansion, EN-1 classifies every root of the protocol subject: FR-120's `domains()` under the request's universes, the record types of every binder ST-2 to ST-4, ST-9, ST-11 and ST-13 range over, and each replicated role's population universe. An unbounded root returns `requires-bound` (ADR-018 §3), which the caller answers with a universe or a bounded domain. |
| TS-6 | **The product.** EN-1 is unchanged: the product of `ProtocolSystem` with the property automaton, keyed by (protocol state key, automaton state index), explored by FR-101's engine, with edge retention, the SCC phase, the fairness filter over PA-1's classes, the limits and the canonical counterexample of ADR-018 §3 and CX-5. |

#### 4.1 Counterexamples and replay

| ID | Rule |
| --- | --- |
| PX-1 | **Content.** A counterexample over a protocol subject is ADR-018's `TemporalCounterexample` (CX-2): the subject with its protocol clause and bindings, the index of the initial state, and each step as its protocol transition identity (TS-2) with its post-state's `quire.simulation.state-key/v1` digest. A deadlock counterexample adds PD-4's blocked threads. The rest points, binders, queues and memory component of each position are data for the reader; the digest already binds them. |
| PX-2 | **Replay.** `qsl_replay::replay_model_trace` replays it, with `ReplaySource::ModelTrace` naming a protocol subject. It recompiles the package (FR-098), re-admits the initial state, universes and bindings, builds `ProtocolSystem` and re-executes each step with FR-101 `replay`. ADR-018 CX-3's refusals apply: a step whose identity has no successor with the recorded digest refuses `stale_dependency`/`content-mismatch`, and a step that is not enabled, or a loop whose last state is not its entry, refuses `invalid_runtime_input`/`invalid-value`. A lasso is checked against the fairness set over PA-1's classes. The formula is then evaluated by ADR-018 SM-1. For a deadlock counterexample, replay enumerates the protocol system's steps at the last state, checks that none is enabled and that some instance is live, and recomputes PD-4's list; agreement settles `reproduced-with-evaluated-witness`, and an enabled step or a state with no live instance settles `inconclusive`, `ReplayParity` (ADR-018 DL-5). |

### 5. Footprints and independence

A **location** is ADR-021 POR-1's model location, (object, field), (object
type, field, any object) or (population, membership), or one of these
protocol locations: `ctl(t)`, thread `t`'s rest point and disposition;
`inst(i)`, parallel instance `i`'s record; `bind(n, t)`, binder `n` of thread
`t`; `queue(h)`, channel `h`'s queue (per FIFO key for `fifo`); `reg(c, ·)`,
template `c`'s registrations in a protocol instance; `role(r)`, replicated
role `r`'s instances in a protocol instance; `pinst`, the set of live protocol
instances; and the locations a memory model names.

| ID | Rule |
| --- | --- |
| FT-1 | **Footprints per step.** Each step `s` of the protocol system has a static write footprint `W(s)` and read footprint `R(s)`. `attempt(n)` by `t`: `R` is the model read footprint of its application (POR-1) with the contract clauses `n` selects, the binders `n`'s constraint reads, and `ctl(t)`; `W` is the frame's model locations (POR-1, POR-3), `bind(n, t)` and `ctl(t)`, and `inst(i)` when the step ends `t`'s branch body. `event(n)` and `finish`: `R` the binders read, `ctl(t)`; `W` `bind(n, t)`, `ctl(t)`, and `inst(i)` when the step ends a branch. `send(n)` and `receive(n)`: as `event(n)`, plus `queue(h)` in both `R` and `W`. `duplicate(h, e)` and `lose(h, e)`: `queue(h)` in both. `fork(p)` by `t`: `R` and `W` `ctl(t)`; `W` the new instance and its threads' `ctl`. `join(p)` by `t`: `R` `inst(i)` and `ctl(t)`; `W` `ctl(t)`, `inst(i)`, and under `outstanding cancel` the `ctl` of every branch thread of `i`. `timeout(a)`: `ctl(t)` in both. An event step whose node is a template's forward effect also writes `reg(c, ·)` (CP-1); a step that binds a record of a template's trigger type reads and writes `reg(c, ·)` of that template and writes the new compensation thread's `ctl` (CP-2); a step of a template's commit node writes `reg(c, ·)` (CP-4). `cattempt(c, r)`: as `attempt`, with `ctl` of its thread, its count and the previous attempt's binder. `cend(c, r)`: its thread's `ctl` and the model locations its recovery predicate reads. `activate(T)`: `W` `pinst`, the new instance's `ctl`. `spawn(r, o)` and `retire(r, o)`: `role(r)` in both, plus the model and binder locations `p` reads for `retire`; a step `by` a replicated role reads `role(r)`. A memory step: its model's footprint. A gate adds the memory locations its model names to `R`. Settling (FO-1) reads only binders and the thread's own control, so its reads are inside these footprints. |
| FT-2 | **Enforced.** Model reads and writes are enforced as ADR-021 POR-2 states (the restricted observation and `check_frame`). Protocol locations are enforced by construction: a step reads and writes the control of its own thread, the queue of its own node's channel, the binders S3 resolves for its node, and the instance its fork or join names, and nothing else. |
| FT-3 | **Enabling footprint.** Each step's footprint is the state-space reduction record's `Footprint{reads, writes, enabling}` (ADR-021 EI-4, FR-155), with the protocol locations of FT-1 as `Location::Protocol(ProtocolLocation)`, `ProtocolLocation` one of `ctl(t)`, `inst(i)`, `bind(n, t)`, `queue(h)`, `reg(c, ·)`, `role(r)` and `pinst`. For a step, the enabling footprint holds the model locations its precondition reads, the pre-state locations its postconditions read (an attempt is enabled only when a post-state exists), the membership of its receiver and of each reference argument, the binders its arguments, constraint, guard or retry relation read (for an `attempt`, every binder an argument expression reads), its own `ctl(t)` and the instance locations it reads (`inst(i)` for a `join`, `reg(c, ·)` and the commit's occurrence for a `cattempt`, `pinst` for `activate`), `queue(h)` for a `receive`, a `send` under `block`, and `duplicate` and `lose` on `h`, `role(r)` for a step `by` a replicated role and for `spawn`, and its gate's memory locations. A disabled step can be enabled only by a step that writes one of these: its own thread, a writer of its precondition's model locations, a step that binds a binder it reads, a sender or remover on its channel, a step that ends a branch of its instance, a registration or activation step, a spawn, a finishing or activating step, or a memory step. This is the necessary enabling set ADR-021 POR-7 takes. |
| FT-4 | **Independence.** Independence is ADR-021 POR-4 over FT-1's footprints. Two steps of one thread share `ctl(t)` and are always dependent. Steps of distinct threads are independent when their model, binder, queue and instance footprints are disjoint in POR-4's sense: two attempts in sibling branches on different objects, or on one object's different fields under receiver-scoped frames, are independent. A `fork` or a branch's internal step is independent of every step of another thread that touches neither its instance nor its locations. |
| FT-5 | **Visibility.** A step is visible (ADR-021 POR-5) when its write footprint meets a model location an atom reads, or when an atom reads the anchor of its position and names its node or operation. `fork`, `join`, `timeout`, channel steps, `event` steps and `finish` write no model location, so they are invisible to every claim whose atoms read model state only. |

### 6. Protocol steps in a refinement

| ID | Rule |
| --- | --- |
| PR-1 | **A protocol as the concrete side.** When the concrete subject of a refinement declaration is a protocol subject, every concrete step class needs an explicit `step` row, as the refinement record requires of every concrete operation. An attempt's steps map by their operation's row, or by a row naming the attempt node, `step <C>::<protocol>::<node> -> …`, which takes precedence over the operation's row for that node; a `cattempt` maps by its operation's row or a row naming its template. Every step with no operation needs its own row: a node row for `event`, `send`, `receive`, `fork`, `join`, `timeout` and `finish` steps, a template row for `cend`, a channel row for `duplicate` and `lose`, a role row for `spawn` and `retire`, a trigger row for `activate`, and a memory-step row, naming the step kind, for each memory step its model has. Each right side is written out: an abstract operation application, `stutter` or `any`. A concrete step class with no row is a compile error at S3, `missing_declaration`/`missing-name`, naming the node, template, channel, role, trigger or memory step kind; two rows for one class refuse `invalid_model_binding`/`conflicting-binding`. Nothing defaults to `stutter` or `any` (QSpec FR-432). |
| PR-2 | **A protocol as the abstract side (QSpec FR-177).** An abstract protocol subject's state is (model state, protocol state). The mapping defines its visible model fields; its rest points, binders, queues and instances are hidden fields, tracked by the refinement record's set of consistent abstract states. Each concrete step's explicit row (PR-1) states its observation: a row to an abstract node or operation makes the step visible, with the **observation label** of that abstract step (its kind, node or operation, and the binder record its row's arguments give); a `stutter` row makes it internal; an `any` row admits either. No concrete step is internal or visible by rule. A visible concrete step passes when an abstract step with its label is enabled from a state of the set after zero or more abstract internal steps, and the set after it is closed under abstract internal steps. An internal concrete step passes when the mapped visible fields are unchanged, and leaves the set unchanged, closed the same way. Which abstract steps are internal is stated the same way, by the refinement profile's explicit list (QSpec FR-177: "the relation declares permitted internal steps"); an abstract step the list does not name is visible. Divergence, refusal sets, terminal success and assumption weakening are QSpec FR-177's further checks on the same product. |
| PR-3 | **Paired initial states.** The initial protocol states pair by FO-3: each concrete initial state maps to an abstract initial model state, and the abstract root starts settled from its `run`. |

### 7. Worked example: two branches and a join

All counts below are hand enumeration; no QSL engine produced them.

**Model.** Object type `Cell` with `v: Int[0, 3]`, population `cells`,
universe `{c}`, one initial snapshot with `c.v = 0`. Operation `inc()`, frame
`modifies self.v`, precondition `IncPre: self.v < 3`, postcondition
`IncPost: self.v = pre(self.v) + 1`. Operation `setTwo()`, frame `modifies
self.v`, precondition `TwoPre: self.v = 0`, postcondition `TwoPost: self.v =
2`. Neither returns a result.

**Protocol.** The spelling is illustrative; QSpec's shared grammar owns it.

```text
protocol Fill using p over (k: M::Cell) on origin {
  role w on M;
  run sequence Main {
    parallel Both {
      branch left  attempt A by w on M::Cell::inc    contracts [IncPre, IncPost] as (a) { true };
      branch right attempt B by w on M::Cell::setTwo contracts [TwoPre, TwoPost] as (b) { true };
    } join all [left, right];
  }
  finish Done as (d) { true };
}
```

The subject binds `k` to `c`. The finish record type has one value. No node
after `A` or `B` reads `a` or `b`, so each binder is removed in the step that
binds it (PS-5), and every state's `observations` member is empty.

**States.** Write a state as (`c.v`; root; `left`; `right`), with `fork`,
`join` and `finish` for the root's rest point and `A`, `B` or `end` for a
branch's.

| State | `c.v` | Root | `left` | `right` | Enabled steps |
| --- | --- | --- | --- | --- | --- |
| s0 | 0 | at `fork Both` | none | none | `fork(Both)` |
| s1 | 0 | at `join Both` | at `A`, running | at `B`, running | `attempt(A)` to s2, `attempt(B)` to s3 |
| s2 | 1 | at `join Both` | end, completed | at `B`, running | none |
| s3 | 2 | at `join Both` | at `A`, running | end, completed | `attempt(A)` to s4 |
| s4 | 3 | at `join Both` | end, completed | end, completed | `join(Both)` to s5 |
| s5 | 3 | at `finish` | none | none | `finish` to s6 |
| s6 | 3 | none: the instance finished and was removed | none | none | none |

Seven states and six transitions. At s1 both orders of `A` and `B` are
enabled: the two branches interleave (CI-3). The instance `Both` is removed
at s5, once its join has settled and both branches have completed (PS-4).

**Terminal states.** s6 holds no live instance, so it is intended (PD-1).
s2 is terminal with its instance live: `right` waits at `B` because `TwoPre` is false at
`c.v = 1`, and the root waits at `join Both` because `join all` needs
`right`. s2 is a deadlock (PD-2).

**Deadlock-freedom.** The request writer adds the subject's deadlock-freedom
item (PD-3). EN-1 decides it in the first phase and settles it `refuted`,
basis `decisive-counterexample` (ADR-018 V-4). s2 is the only deadlocked
state, so the canonical counterexample is the path to it in either
canonical order of `A` and `B`:

| Position | State | Step into it |
| --- | --- | --- |
| 0 | s0 | initial |
| 1 | s1 | `fork(Both)`, thread root |
| 2 | s2 | `attempt(A)`, thread `left`, `M::Cell::inc` on `c` |

Blocked threads (PD-4): `right` at `B`, `Precondition{TwoPre}`; root at
`join Both`, `JoinWaiting{[right]}`. Replay (PX-2) re-executes `fork(Both)`
and `attempt(A)` through `ProtocolSystem`, checks each post-state digest,
finds no enabled step at s2 and its instance live, recomputes the
blocked list and settles `reproduced-with-evaluated-witness`.

**A claim at the deadlock.** `eventually holds(k.v = 3)` under infinite-trace
is TP-4. The behaviour s0, s1, s2 ends at a terminal state, so it is
stutter-extended (PB-4), and `c.v` stays 1. EN-1 settles the claim `refuted`
with the lasso whose prefix is s0, s1 and whose loop is s2's terminal stutter
step, as ADR-018 DL-6 states. The deadlock-freedom item reports the same
state.

**The repaired protocol.** With `TwoPre: self.v <= 1`, `attempt(B)` is
enabled at s2 and reaches (`c.v` = 2; join; end; end). The reachable states
are s0, s1, s2, s3 and, after both branches, two join-pending states (`c.v`
2 and 3), two finish-pending states and two states whose instance finished and was
removed: ten states and nine transitions. Both terminal states hold no live
instance, so the deadlock-freedom
item settles `proved`, `Proved{basis: Exhaustive}` (ADR-018 V-1), and
`eventually holds(k.v = 3)` settles `refuted` with the finished behaviour
through `c.v = 2`, since `B` after `A` leaves 2.

**Fairness.** The clause carries the scheduler constraints `fair weak whole`
over `left`, `right` and the root `Fill` (PA-3). The deadlock lasso's loop is
the terminal stutter step, which is outside every constraint, so they do not
change the verdict. `fork`, `join` and `finish` are uncounted (PB-3): along
s0 to s6 through `B` first, the counted steps are `attempt(B)` and
`attempt(A)`, so `eventually[0,2] holds(k.v = 3)` holds at position 0.

**Independence.** `A` and `B` both write (`c`, `v`), so they are dependent
(FT-4) and ADR-021 POR keeps both orders. Had `B` written another object's
field under a receiver-scoped frame, the two attempts would be independent,
and for a claim that reads neither field POR would expand one order at s1.

#### 7.1 A compensation

**Model.** Object type `Acct` with `bal: Int[0, 1]`, universe `{x}`, initial
`x.bal = 0`. Operation `charge()`, precondition `self.bal = 0`,
postcondition `self.bal = 1`; operation `refund()`, precondition `self.bal =
1`, postcondition `self.bal = 0`; both with frame `modifies self.bal`. Record
type `Failure` with field `failed: Boolean`. The effect and finish record
types have one value each.

```text
protocol Pay using p over (k: M::Acct) on origin {
  role w on M;
  compensate Refund for Main::Paid as (fwd: M::ChargeEffect)
      by w on M::Acct::refund using p clock "c" {
    activate first (f: M::Failure) when { f.failed } { }
    within [0, 10];
    attempts 1 of M::RefundAttempt;
    retry (e: M::RefundAttempt, l: M::RefundAttempt) { false };
    commit never;
    recover (r: M::RecoveryView) { r.bal = 0 };
  }
  run sequence Main {
    attempt C by w on M::Acct::charge contracts [ChargePre, ChargePost] as (c) { true };
    effect Paid of Main::C as (pd: M::ChargeEffect) { true };
    event Fail by w as (f: M::Failure) { true };
  }
  finish Done as (d) { true };
}
```

| State | `x.bal` | Root | Registration `Refund` | Compensation thread | Enabled steps |
| --- | --- | --- | --- | --- | --- |
| t0 | 0 | at `C` | none | none | `attempt(C)` |
| t1 | 1 | at `Paid` | none | none | `event(Paid)` |
| t2 | 1 | at `Fail` | registered | none | `event(Fail)` with `failed` false to t3, true to t4 |
| t3 | 1 | at `finish` | registered | none | `finish` to t5 |
| t4 | 1 | at `finish` | activated | at its attempt, count 0 | `cattempt(Refund)` to t6 |
| t5 | 1 | none: removed | none: removed with the instance | none | none |
| t6 | 0 | at `finish` | activated | at its retry point, count 1 | `cend(Refund)` to t7 |
| t7 | 0 | at `finish` | recovered | none | `finish` to t8 |
| t8 | 0 | none: removed | none: removed with the instance | none | none |

Nine states and eight transitions. `event(Paid)` registers the compensation
(CP-1). `event(Fail)` with `failed` true activates it in the same step (CP-2),
creating the compensation thread; with `failed` false the registration stays
`registered` until the instance finishes and is removed with it (CP-5, AE-3). At t4 `finish` is
not enabled, because the compensation thread is running (ST-9). At t6 the
count has reached `attempts 1` and the retry relation is false, so `cend` is
the only step (CP-5), and the recovery predicate `bal = 0` holds. Both
terminal states hold no live instance, so the deadlock-freedom item settles
`proved` (V-1).

Counted steps on the failure path are `attempt(C)`, `event(Paid)`,
`event(Fail)` and `cattempt(Refund)`; `cend` and `finish` are uncounted
(PB-3). From position 2 (t2, `bal = 1`) the distance to t6 is 2, so
`eventually[0,2] holds(k.bal = 0)` holds at position 2 of that behaviour and
fails at position 1, whose distance to t6 is 3. On the success path t0,
t1, t2, t3, t5, which never refunds, it fails at every position from 1.

### 8. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | The protocol subject (PS-1): protocol clause, `over` and role bindings, replicated role universes, one instance per initial state under `on origin` and one per trigger under `on each` (AE-1 to AE-3), the protocol's steps as the only steps | QSpec FR-425 |
| QS-2 | The step kinds ST-1 to ST-15, their enabledness and effects, folding of structural moves (FO-1, FO-2), atomicity (AT-1 to AT-3), and the untimed reading of `await` | QSpec FR-426, FR-052 |
| QS-3 | The content of the `control`, `observations`, `bounds`, `queues` and `roles` members of the state key, the thread path and ordinal (PS-2), and the removal rules of PS-4 and PS-5 | QSpec FR-425 |
| QS-4 | The protocol transition identity's typed canonical form (TS-2) | QSpec FR-426 |
| QS-5 | The causal-interleaving statement CI-3 as the meaning of FR-052's "every interleaving that preserves each branch's causal edges" | QSpec FR-052 (causal interleavings), FR-052-AC-6 |
| QS-6 | Fairness targets for attempt nodes and branches, and their granularity (PA-1); the `protocol` anchor kind (PB-2) | QSpec FR-430, FR-431 |
| QS-7 | Intended terminal states and deadlocks of a protocol subject (PD-1 to PD-4), with the wait causes | QSpec FR-429 |
| QS-8 | The counterexample wire for protocol steps and blocked threads, and their replay (PX-1, PX-2) | QSpec FR-433 |
| QS-10 | Compensation in a model subject: registration on the forward effect, activation on the first eligible trigger, compensation attempts under the authored count and retry relation, the commit cutoff and `cend` with its recovery status (CP-1 to CP-5) | QSpec FR-427 |
| QS-11 | Replicated roles: instances as population objects, spawn under the authored `max`, lifetimes and retirement, the object reference as the state key's identity with FR-171's ordinal as trace data (RR-1 to RR-4); activation on each with its instance identity and the `max_live_instances` bound with its default of 3, its V-6 cause and its statement in the terminal record (AE-1 to AE-4) | QSpec FR-428 |
| QS-12 | Counted and uncounted steps, and interval distance in counted steps over a protocol subject (PB-3) | QSpec FR-431 |
| QS-13 | The protocol-level `terminal` member (PD-1) and the `scheduling adversarial` member with the scheduler constraints and their `scheduler` origin (PA-3) | QSpec FR-429, FR-430 |
| QS-9 | Conformance vectors, each with expected verdict and, for a refutation, a counterexample that must replay: (a) §7, its deadlock, its claim and its repair, and §7.1 with both interval verdicts; (b) QSpec FR-052-AC-1's split shipments, with every interleaving reached; (c) each join policy at zero, below-threshold, threshold and post-threshold dispositions, under `continue` and `cancel` (QSpec FR-171-AC-4); (d) a `parallel` nested three deep inside a `repeat` with `outstanding continue`, showing ordinal reuse; (e) a `fifo` and an `unordered` channel under each delivery policy; (f) an `on each` protocol under `max_live_instances` 1 and 2, settling V-6 and the PD-2 activation-only deadlock; (g) a replicated role with each lifetime, showing that a retired instance is not spawned again; (h) a liveness claim that holds under the scheduler constraints and is refuted under `scheduling adversarial` | QSpec TC-375 to TC-387 |

### 9. Downstream impact and sequencing

**Stage DAG (ADR-011 §1).** No stage or edge is added. E10 carries a
protocol subject as well as a model subject. S3 extends its protocol checks to
every construct the protocol system admits (ADR-017 PF-7, scenario 1). Crate
`qsl-eval`'s `simulation` gains `ProtocolSystem`; the `model_check` engine in
crate `qsl-analyze` (ADR-029) drives it as it
drives `ModelSystem`; layer 6 `replay` re-executes through it.

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: check `parallel`, `branch`, `join` with its policy and outstanding rule, `choice`, `repeat`, `await`, `check`, `send`, `receive`, `effect`, `event` and `commit`, compensation templates, replicated roles, `activation on each`, the protocol `terminal` and `scheduling` members, and the scopes PS-5 and RR-4 read. FR-120: expose the one application function TS-3 calls. `qsl-eval`'s `simulation`: `ProtocolSystem`, `MemoryModel` with `Sc`, PS-8's key. `model_check` (`qsl-analyze`, ADR-029): the protocol subject in the pre-check (TS-5), PA-1's classes and PA-3's scheduler constraints in the fairness filter, PD-2's `deadlocked`, counted-step distance in the interval translation (PB-3), the instance bound and `InstanceBoundReached` (AE-4). `qsl-replay`: PX-1 and PX-2. |
| DS-2 | CG | The negotiation arm reads the subject kind, model or protocol. |
| DS-3 | IR | With ADR-018's SMT path: the protocol system's transition relation. |
| DS-4 | QSpec | §8. |

**Sequencing in QSL.**

1. **Specification.** This record, QSpec §8 and the QSL compiler
   requirements FR-205 to FR-218.
2. **Prerequisites.** ADR-018 EN-1 and FR-120 `ModelSystem`; S3 checks of the
   constructs in DS-1.
3. **`ProtocolSystem` under `sc`** with `sequence`, `parallel` and its join
   policies, `attempt` and `finish`; §7 is its first conformance vector.
4. **The other constructs**: `choice`, `repeat`, `check`, event nodes,
   channels and `await`; then compensation templates, replicated roles and
   `activation on each`, with §7.1 as the compensation vector.
5. **The dependent records**: POR over `parallel`, protocol refinement and the
   weak memory models, each on this system.

### 10. Rulings on the draft's questions

The owner ruled on the five questions the draft left open, on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Design compensation templates, replicated roles and per-trigger activation here, or in a follow-up | **Design all three in this record.** No construct of the protocol language settles `unsupported` for want of a design | Specifications are completed up front; a protocol subject that refused these constructs would leave the language's own compensation and dynamic-role semantics without a checker | §2.2a AE-1 to AE-4, RR-1 to RR-4, CP-1 to CP-5; ST-11 to ST-15; §7.1 |
| RU-2 | Whether protocols have a deadlock opt-out | **Deadlocks are reported by default, with a per-protocol opt-out, as ADR-018 does for models.** `terminal when P` marks intended terminal states; `terminal any` opts out | The same reasoning as ADR-018 RU-3: a stuck state is usually a defect, and a protocol that stops on purpose in some states says which | PD-1 to PD-3 |
| RU-3 | Whether fair scheduling is assumed | **On by default**: one weak `whole` constraint per branch, the root and each compensation template, which a protocol can turn off to check against an adversarial scheduler | A protocol's branches are run by a scheduler that serves each runnable thread eventually; liveness claims that fail only by starving a thread report the scheduler, not the protocol. The constraints are derived, carry a `scheduler` origin and enter the obligation identity, so the premise stays visible | PA-3; §7 |
| RU-4 | Whether fork, join and finish count for step-counted intervals | **They do not.** Only operations, events, sends and receives count | This keeps TLA+'s stutter-invariance spirit: control bookkeeping does not move a bounded clock. A timed model counts time instead | PB-3; SE-3; §7, §7.1 |
| RU-5 | Whether control steps map to `stutter` by rule in a refinement | **No: every step class keeps an explicit row**, control steps included, as the owner ruled for the refinement record (ADR-020 RU-1) | A default lets an unintended step pass unchecked | PR-1, PR-2 |

## Consequences

- A `parallel` protocol becomes a model subject, so every ADR-018 verdict,
  fairness constraint, deadlock report and counterexample applies to it.
- QSpec FR-052's causal interleavings have one operational definition, and
  the explicit-state engine enumerates exactly those interleavings that the
  model's contracts allow.
- Attempts keep FR-120's meaning: one atomic operation application, shared
  code with `ModelSystem`.
- A stuck join is a deadlock with a counterexample that names each waiting
  thread and why it waits.
- Every step is a position, but interval operators over a protocol count only
  operations, events, sends and receives.
- Liveness over a protocol assumes a fair scheduler unless the protocol opts
  out, and the assumption is part of every obligation identity and
  counterexample. Partial-order reduction for liveness then needs a
  preservation row for scheduler fairness (to make on acceptance, ADR-021).
- Compensation, replicated roles and per-trigger activation are checked with
  the rest of the protocol. Per-trigger activation can grow without end;
  `max_live_instances`, default 3 and settable by the caller, bounds it and
  turns a pass that reached the bound into `inconclusive`, with the value used
  stated in the result.
- Steps carry static footprints, so partial-order reduction over sibling
  branches has its independence relation, and a weak memory model plugs in
  through one seam without changing the protocol rules.
- The untimed reading of `await` over-approximates timed behaviour: a proof
  holds for every deadline, and a refutation that relies on a timeout may
  need a deadline the timed reading excludes.
- Interleaving costs state space exponential in the number of concurrently
  enabled threads; POR is where that cost is addressed.

## Amendments made with this record

Each amended cell carries an "Amended by ADR-027" note.

- ADR-018 §2 SM-2: a protocol subject is a model subject. SM-3: positions of a
  protocol subject are its steps, with `protocol` anchors. SM-4: a state whose
  instances have all finished and been removed is terminal. §4 FA-1: constraints over a protocol subject may name
  an attempt node or a branch. FA-2: enabledness of a protocol step class.
  §10 DL-1: a protocol subject's intended terminal states are those with no
  live instance and those its protocol-level `terminal` member marks. DL-2:
  deadlocks of a protocol subject, reported unless it declares `terminal
  any`. FA-1 also: the scheduler constraints. §11 IV-2: over a protocol
  subject, interval distance counts counted steps only. §5 CX-2: protocol transition
  identities and blocked threads. CX-3: replay through `ProtocolSystem`.
- ADR-018 §1 V-6: `InstanceBoundReached{bound, states}` (AE-4). §4 FA-5:
  the constraint's target is one of PA-1's targets, and each constraint
  carries its origin (`authored`, `scheduler` or the weak-memory record's
  `memory`).
- ADR-016 FP-4: `ProtocolSystem` is a second production `TransitionSystem`
  beside `ModelSystem` (TS-1).
- ADR-017 PF-7: a fifth change scenario, the protocol transition system.
- ADR-011 §2.1 E10: the subject may be a protocol subject. §6.2: `qsl-eval`'s
  `simulation` holds `ProtocolSystem` and `MemoryModel`; the `model_check`
  engine (`qsl-analyze`, ADR-029) drives
  either system.
- `spec/spec.md`: the index row for this record.

## Amendments to make on acceptance

These records are drafts on other branches; each edit carries an "Amended by
ADR-027" note when made.

- **ADR-020 (refinement).** RM-5: over a protocol subject, RU-1's rule covers
  every concrete step class, not only operations. A `step` row may name an
  attempt node, which takes precedence over its operation's row; every step
  with no operation (`event`, `send`, `receive`, `fork`, `join`, `timeout`,
  `finish`, channel `duplicate` and `lose`, memory steps) needs its own node,
  channel or memory-step row, with `-> op`, `-> stutter` or `-> any` written
  out; an unmapped class is a compile error (PR-1). §7 MC-1 and the protocol
  column of its comparison table: the protocol `TransitionSystem` is
  `ProtocolSystem`; observation labels come from each step's explicit row, and
  internal steps from the explicit row or the profile's explicit list (PR-2);
  the abstract protocol's control state is hidden and tracked by AX-2's set,
  closed under abstract internal steps.
  RC-1 and RC-2: a concrete protocol counterexample carries PX-1's steps and
  replays by PX-2. RM-5 also: template, role and trigger rows for `cend`,
  `spawn`, `retire` and `activate`. CO-4: over a protocol subject an interval
  counts counted steps (PB-3). RM-6: the concrete fairness set `F_C` over a
  protocol subject includes the scheduler constraints (PA-3).
- **ADR-021 (state-space reduction).** POR-1: footprints include the protocol
  locations `ctl`, `inst`, `bind` and `queue`, with FT-1's per-step footprints.
  POR-7: the necessary enabling set of a disabled protocol step is FT-3's.
  POR-9: `ProtocolSystem` offers POR with FT-1 to FT-5. POR-5: control steps
  are invisible to model-state atoms (FT-5). Its Context bullet "`parallel` is
  a protocol construct" names this record. PT-2 rows over a protocol subject
  are the same as over a model subject; a branch fairness constraint (PA-1)
  reads as `whole` or `each` as an operation constraint does. The scheduler
  constraints (PA-3) make every liveness clause's fairness set non-empty, so
  its "TP-4, weak, every constraint `whole`" row reads "no" for POR over a
  protocol subject by default; a protocol with `scheduling adversarial` and no
  authored constraint keeps the empty-set row. A preservation row for POR
  under one weak `whole` constraint per thread is the reduction record's to
  design. The footprints add `reg`, `role` and `pinst` (FT-1).
- **ADR-025 (weak memory).** Made in ADR-025 itself, each item with an
  "Amended by ADR-027" note. Context: the protocol system is this record's
  `ProtocolSystem`, and its open question 1 is answered by it. SC-1: `sc` is
  SE-2. TSO-7 and RA-6: fork, join and `send` gates and the split and merge
  are SE-1 (e) and (f). MS-1: the memory component is SE-1 (a), the state key
  member PS-8 names. MS-2: memory steps are positions with their own anchor
  (PB-2, SE-3). MA-5: an access evaluates over SE-1 (b) and writes through
  SE-1 (c). MA-6: guards and checks read binders and constants only (FO-2),
  so its rule reduces to binders. MB-1: "attempts, fences, controls and memory
  steps" are ST-1 to ST-15 plus the fence step. MF-4: authored constraints
  read PA-1's classes, of which memory steps are in none. MR-1 and MK-1:
  memory footprints and identities sit beside FT-1 and TS-2. §11
  (refinement): each memory step kind has an explicit memory-step row naming
  it, with `-> any` written out where that is the intent (PR-1, QSpec
  FR-432). MS-2: memory and fence steps are uncounted for intervals
  (PB-3, SE-3, QSpec FR-431). MF-1 and MF-2: the memory constraints sit beside the
  scheduler constraints (PA-3) in every fairness set.
- **ADR-019 (strong fairness).** SR-1: the enabled set of a protocol state is
  the set of its enabled protocol transition identities, read by PA-1's
  classes; the scheduler constraints are weak rows in its filter.

## Alternatives Considered

- **Every structural move a step.** Rejected. Entering a sequence, selecting a
  case and leaving a repeat change no model state and are determined by
  immutable binders (FO-2). As steps they would add positions and multiply
  interleavings with no new behaviour.
- **Fork and join folded into neighbouring steps.** Rejected. A weak memory
  model gates both (SE-1 e), and `outstanding cancel` makes the join an
  explicit transition (QSpec FR-228). Keeping them as steps under every model
  gives one protocol system whose models differ only through the seam.
- **Branch termination as a step.** Rejected. Ending a branch changes only its
  disposition, which its last step sets; the join, which reads it, is the step.
- **Attempts split into invoke and complete.** Rejected. An operation contract
  relates one pre-state to one post-state; an invocation that reads at one
  position and writes at another has no FR-120 meaning, and every per-function
  proof (ADR-017 AR-2) is over the whole operation. Finer interleaving inside
  an operation is a memory model's business (SE-1), stated per access.
- **A disabled attempt fails instead of waiting.** Rejected. Blocking is
  FR-120's enabledness, so ADR-018 FA-2 and DL-2 read it unchanged, and a
  stuck join is reported as a deadlock with its cause (PD-4). A protocol that
  wants an explicit failure path models it with a result and a choice.
- **Steps with no operation mapped by default in a refinement** (`any` or
  `stutter`). Rejected, as the refinement record's RU-1 rejects an unmapped
  operation defaulting to `any`: a default lets an unintended step pass
  unchecked. Every concrete step class has an explicit row (PR-1).
- **No default scheduler fairness.** Rejected (RU-3). The derived scheduler
  constraints keep the premise in the clause's fairness set, its identity and
  its counterexample, which is what ADR-018 asks of a premise.
- **Control steps counted for intervals.** Rejected (RU-4).
- **Replicated role identity by spawn ordinal in the key.** Rejected. Spawn
  order would split equal instance sets into distinct states; the object
  reference is canonical, and the ordinal stays trace data (RR-2).
- **No instance bound for `on each`, or a fixed one.** Rejected. Unbounded,
  an always-enabled trigger makes every run stop on EN-1's state limit (V-7);
  fixed, a caller could not check more instances. A caller-set bound with a
  small default, stated in every result, is how the search horizon and the other method bounds work
  (AE-4).
- **Retries forced while the retry relation allows one.** Rejected. QSpec
  FR-058 has the retry relation constrain attempts, never require them; `cend`
  stays enabled (CP-5).
- **Branch identity by fork count.** Rejected. A counter that grows with each
  fork makes every loop that forks infinite-state. PS-2's lowest free ordinal
  gives a finite key whenever the number of live instances is finite.
- **A clocked reading of `await`.** Not in this record: model behaviours carry
  no clock value (ADR-018 §1). The dense-time record (References) owns a timed
  reading.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-425 to FR-433, with
  the causal interleavings in QSpec FR-052 (FR-052-AC-6) and the loop rule
  in QSpec FR-052 (Linear STD-140). Built on ADR-018 (Linear QSL-366).
- Records that build on this one: ADR-020 refinement mappings (Linear
  QSL-367), ADR-021 state-space reduction (Linear QSL-368), ADR-025 weak
  memory models for `parallel` (Linear QSL-372). Related: ADR-019 strong
  fairness (Linear QSL-365), ADR-026 dense time (Linear QSL-373).

---
id: ADR-025
title: "Weak memory models for parallel"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-431
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-432
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-434
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-435
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-436
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-437
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-438
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-439
    type: depends_on
---
# ADR-025: Weak memory models for parallel

## Status

Proposed, 2026-10-01. §13 records the owner's rulings on the six questions
this record first left open; §14 and §15 carry the two that add design. The
QSL compiler requirements that implement it are FR-219 to FR-229, traced to
US-025; QSpec's half is listed under References.

It builds on ADR-018, itself a draft, and follows the owner's rulings recorded
there (ADR-018 RU-1 to RU-4). It depends on ADR-027, the protocol
transition system, for protocol steps: the states, steps and transition
identities of a protocol run, with attempts applied through `ModelSystem`.
This record adds the memory model to that system. It uses the static
footprints and receiver-scoped frame entries of the state-space reduction
record (ADR-021 POR-1, POR-3), a draft. It reads the strong-fairness
(ADR-019), refinement (ADR-020), reduction (ADR-021), possible-property
(ADR-022), hyperproperty (ADR-023) and statistical (ADR-024) records, all
drafts, for their interactions (§11). The owning ticket and related work are
listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `MM-`, `MA-`, `SC-`, `TSO-`,
`RA-`, `MS-`, `MF-`, `MB-`, `MR-`, `MX-`, `MV-`, `MK-`, `DS-`, `RU-`, `DR-`
and `PSC-` are local to this record. Other artifacts cite them as
`ADR-025 TSO-3`. Items of ADR-018 are cited as `ADR-018 SM-3`.

## Context

**What `parallel` means today.** QSpec FR-052 states that "Parallel branches
SHALL admit every interleaving that preserves each branch's causal edges", and
QSpec FR-228 lists `parallel-branch` and `join` among the admitted control
constructs. The compiled protocol form is `{kind:"parallel", branches,
join}` (docs/compiled-protocol-v1.md), with the structural edges in → each
branch and every branch → out. A branch holds protocol events. An `attempt`
event invokes an operation of a role's model; its effect on model state is
FR-120's successor rule: an operation applied to an argument vector, enabled
when its effective precondition holds, times every post-state in its frame
that satisfies its postcondition. Each event is one indivisible step of the
interleaving. This is sequential consistency (Lamport) at the granularity of a
whole operation: every step sees every earlier step's writes, in one total
order.

**What explores it.** FR-120's `ModelSystem` is QSL's only production
`TransitionSystem` (ADR-016 FP-4), and a protocol system waits on protocol
nodes at S4 (ADR-017 PF-7). ADR-027 specifies that system once,
`ProtocolSystem<M: MemoryModel>`, as the one foundation ADR-020, ADR-021
and this record build on, and answers this record's first open question
(RU-1). This record specifies only what a weak memory model adds to it,
through ADR-027's memory-model seam (ADR-027 SE-1). Amended by ADR-027.

**Why it matters.** TLA+ and PlusCal assume sequential consistency, so an
author who models x86 or C11 code writes the store buffer by hand. The link to
code is the larger issue. QSpec FR-353 and ADR-017 AR-2 bind each model
object's fields to Rust fields and each operation frame to one Rust function.
Kani and Verus then prove each function against its operation's contract,
one function at a time. A model proof under interleaving semantics, together
with those per-function proofs, says something about concurrent code only when
each function's concurrent effect is as indivisible as the model step that
stands for it. Code built on atomics with orderings weaker than `SeqCst` has
behaviours no interleaving of whole operations produces.

**The literature this record uses.**

- **x86-TSO** (Sewell, Sarkar, Owens, Zappa Nardelli and Myreen; Owens,
  Sarkar and Sewell): each hardware thread has a FIFO store buffer; a store
  enters the buffer; a load reads the newest buffered store to its location,
  else memory; buffered stores reach memory one at a time, in order; `MFENCE`
  and `LOCK`-prefixed instructions run with the buffer empty.
- **RC11** (Lahav, Vafeiadis, Kang, Hur and Dreyer): a repaired C11 model with
  non-atomic, relaxed, release, acquire and `seq_cst` accesses and fences. Its
  `seq_cst` accesses and fences are ordered by the **partial SC order**
  `psc`, which must be acyclic. Program order together with reads-from is
  acyclic, which rules out load buffering and out-of-thin-air values. A data
  race on a non-atomic location is undefined behaviour.
- **The promising semantics** (Kang, Hur, Lahav, Vafeiadis and Dreyer): an
  operational model with a message set per location ordered by timestamps,
  per-thread views, views carried on messages, a global view for `seq_cst`
  fences, and promises. Without promises, it is a finite operational model of
  release, acquire and relaxed accesses.
- **GenMC** (Kokologiannakis and Vafeiadis): stateless model checking of C
  code under RC11 and other axiomatic models by enumerating execution graphs.
  It serves here as an independent oracle for the litmus vectors; checking
  lowered code with it is later research (References).
- **Memory fairness** (Lahav, Namakonov, Oberhauser, Podkopaev and
  Vafeiadis): the fairness a weak memory model needs for liveness, so that a
  store is eventually flushed and a write is eventually observed.
- **Race detection** (Flanagan and Freund, FastTrack): a race on a location
  is found by comparing each access with the location's last write and each
  thread's last read, under happens-before.
- **Decidability** (Atig, Bouajjani, Burckhardt and Musuvathi): state
  reachability under TSO with unbounded buffers is decidable only with
  non-primitive-recursive complexity, and harder models are undecidable. An
  explicit-state engine therefore takes a bound on buffers or message sets
  as a method bound.

**What QSL has that bears on this.** ADR-018 gives EN-1, the explicit-state
product over FR-101's canonical breadth-first engine, with weak fairness
(FA-1 to FA-6), the greedy canonical loop (CX-5), deadlocks (DL-1 to DL-7),
the V-1 to V-8 verdicts, and `ModelCheckLimits`, EN-1's limits and method
bounds (IV-6). ADR-021 gives static read and write footprints per transition
identity (POR-1), receiver-scoped frame entries (POR-3) and the preservation
table PT-2. ADR-017 AR-2 gives the code binding.

## Decision

### 1. The memory model is a parameter of `parallel`

| ID | Rule |
| --- | --- |
| MM-1 | **Source default.** A `parallel` control declares its default memory model after its name: `parallel Name memory M { branch … } join all [ … ];`, with `M` one of `sc`, `tso` and `ra`. A `parallel` with no `memory` clause has default `sc`, which is QSpec FR-052's interleaving meaning. `tso` and `ra` are the **weak** models. The spelling is illustrative; QSpec's shared grammar owns it. |
| MM-2 | **Request selection.** A request may select the model of an outermost `parallel`, one nested in no other `parallel`, with a member `memory: [{parallel, model}]` naming the control by its scoped path (FR-112) and the model. The selection replaces the model of that `parallel` and of every `parallel` nested in it. A `parallel` the request names no model for keeps its source default. A selection that names a nested `parallel` or a control that is not a `parallel`, or two entries that name the same `parallel`, refuses the request, `invalid_runtime_input`/`invalid-value`, naming the entry. The **resolved model** of an outermost `parallel` is the request's selection when present, else its source default. The member's spelling is QSpec's (QSpec FR-331). |
| MM-3 | **Scope and nesting.** The resolved model governs its `parallel`'s branches and every control nested in them. Weak `parallel` controls nest to any depth; each nested `parallel`'s branches are threads forked from the enclosing branch. In source, a nested `parallel` declares the same default as its outermost `parallel` or none; another default refuses at S3, `invalid_model_binding`/`conflicting-binding`, naming both controls. A `parallel` whose resolved model is weak joins `all` with no `outstanding` clause; S3 refuses another join policy on a `parallel` that declares a weak default, `unsupported_construct`/`declaration-form` at the join, naming the policy and the weak default, and EN-1's pre-check settles a request that selects a weak model for such a `parallel` V-8, `WeakAccessShape` (MA-4). Sequential `parallel` controls in one protocol may resolve to different models: each `join all` returns memory to one value per location (TSO-7, RA-7). |
| MM-4 | **Threads.** Each branch of a weak `parallel` is one thread. Its program order is its causal order (QSpec FR-052). A control outside every weak `parallel` belongs to the enclosing thread, and its accesses apply in place, as under `sc`. |
| MM-5 | **Identity.** The effective model of every outermost `parallel`, the request's selection or else the source default, is written into the request and enters its obligation identity (ADR-013 O-09) beside the subject (ADR-018 §1 "Scope"), a defaulted model as explicitly as a selected one (QSpec FR-434). The source `memory` clause enters the checked package's identity as source content, and the obligation identity only through the effective model. The state key's `memory` member (QSpec FR-181) holds the component under it. Two requests that resolve every `parallel` to the same models carry the same identity whether the models came from source or from the request. The same protocol checked under `sc` and under `tso` is two obligations, and a verdict on one never joins the other's request (ADR-014 §8). |
| MM-6 | **Orderings.** An `attempt` inside a `parallel` may carry `ordering o`, with `o` one of `relaxed`, `acquire`, `release`, `acq_rel` and `seq_cst`, the C11 and Rust orderings. An access with no `ordering` is **non-atomic** (§14). A new control `fence Name ordering o;` with `o` one of `acquire`, `release`, `acq_rel` and `seq_cst` issues a fence in its thread. The spellings are illustrative (QSpec's shared grammar). |

```text
run parallel Both memory sc {                                // source default
  branch left sequence L {
    attempt Sx by c1 on M::Cell::set  ordering release …;   // x := 1
    fence F1 ordering seq_cst;                               // only in the fenced variant
    attempt Ly by c1 on M::Core::read ordering acquire …;   // c1.r := y
  }
  branch right sequence R { … }
} join all [left, right];
```

```text
memory: [{ parallel: "SB::Both", model: "tso" }]             // request selection, MM-2
```

### 2. Accesses

| ID | Rule |
| --- | --- |
| MA-1 | **Locations and sharing.** A location is ADR-021 POR-1's: (object, field), (object type, field, any object), or (population, membership). EN-1 instantiates each attempt's read and write footprints per transition identity over the subject's universes and argument domains. A location is **shared** in a weak `parallel` when the instantiated footprints of attempts in two or more of its branches contain it and at least one of them writes it. Every other location an attempt touches is **branch-local**: a register of its thread. |
| MA-2 | **Classification.** Each instantiated attempt in a weak `parallel` is one of: a **load** of `ℓ` (its shared footprint is `{ℓ}`, read only); a **store** to `ℓ` (`{ℓ}`, written only, with no pre-state read of `ℓ`); a **read-modify-write** (RMW) of `ℓ` (`{ℓ}`, read in the pre-state and written); or **local** (empty shared footprint). A load or store with an ordering is **atomic**; one without is **non-atomic**. A `fence` control is a **fence**. Branch-local reads and writes of an attempt are part of the same step. S3 classifies every attempt in every `parallel` symbolically, whatever its source default, so that a request selection (MM-2) finds the classification in place. |
| MA-3 | **Admitted orderings.** A load takes `relaxed`, `acquire`, `seq_cst` or none; a store takes `relaxed`, `release`, `seq_cst` or none; an RMW takes one of the five and always has one; a local attempt takes none. An ordering its kind does not take, or an RMW with none, refuses at S3, `ill_typed`/`operator-ineligible`, naming the attempt. Under `sc`, orderings change no behaviour and are recorded for the code link (MX-4). |
| MA-4 | **One location per access.** Under a weak model, an attempt instance whose shared footprint holds two or more locations, an any-object location, or a population membership is not one hardware or C11 access. S3 refuses it when the `parallel` declares a weak default and the receiver and parameters decide it symbolically (two shared fields of the receiver), `unsupported_construct`/`declaration-form`, naming the attempt and the locations. Otherwise EN-1's pre-check, which reads the resolved model, finds it on instantiation and settles the item V-8, `unsupported`, cause `WeakAccessShape{site, transition, shape}` (MV-1). The same cause covers MM-3's join policy under a selected weak model. Under `sc` every attempt is admitted: a whole operation is one step. |
| MA-5 | **Applying an operation under a weak model.** An access applies its operation by FR-120's successor rule over the **thread observation**: the model state with `ℓ` holding the value the memory model gives the reading thread (TSO-3, RA-2), and with branch-local locations as they are. The precondition reads that value, so a load whose precondition constrains `ℓ` is enabled only for values that satisfy it. The postcondition's delta at `ℓ` goes to the memory model (TSO-2, RA-3); branch-local writes apply in place. The thread observation is ADR-027 SE-1 (b), and the delta goes through SE-1 (c). An RMW application whose post-state leaves `ℓ` unchanged is a load with the read part of its ordering, as C11 reads a failed compare-and-exchange. Amended by ADR-027. |
| MA-6 | **Guards and checks.** Under every model, a choice guard, repeat guard or `check` reads binders and constants only (ADR-027 FO-2; QSpec FR-435). One that reads a model location refuses at S3, `ill_typed`/`operator-ineligible` at the read (FR-218). So no guard reads a shared location, and a weak model changes no guard's value. An author who needs a shared value in a guard writes a load attempt and guards on its binder. Amended by ADR-027. |

### 3. Operational semantics

Each model below is an operational model: a memory component added to the
protocol system's state, and rules that say which steps are enabled and what
they do. EN-1 explores it like any other successor relation.

#### 3.1 SC

| ID | Rule |
| --- | --- |
| SC-1 | **Interleaving.** The memory component is empty. Each attempt is one step that applies its operation by FR-120's rule to the model state. This is QSpec FR-052's meaning and today's. `sc` is ADR-027 SE-2, the memory-model seam's identity. Amended by ADR-027. |

#### 3.2 x86-TSO

| ID | Rule |
| --- | --- |
| TSO-1 | **State.** The memory component is one store buffer per thread of the active weak `parallel`: a sequence of (location, value) pairs in program order. Memory is the model state's value of each shared location. |
| TSO-2 | **Store.** A store by thread `b` to `ℓ` with value `v` appends `(ℓ, v)` to `b`'s buffer. Memory is unchanged. |
| TSO-3 | **Load.** A load by `b` of `ℓ` reads the value of the newest `(ℓ, v)` in `b`'s buffer, else memory's value of `ℓ`. |
| TSO-4 | **Flush.** An internal step `flush(b)`, enabled when `b`'s buffer is non-empty, removes its oldest pair `(ℓ, v)` and writes `v` to `ℓ` in memory. It has its own transition identity and no operation. |
| TSO-5 | **Locked accesses.** An RMW, and a store with ordering `seq_cst`, is a locked instruction: enabled only when `b`'s buffer is empty, it reads and writes memory directly in one step. |
| TSO-6 | **Fences.** A `seq_cst` fence is `MFENCE`: enabled only when `b`'s buffer is empty, it changes nothing. An `acquire`, `release` or `acq_rel` fence emits no instruction (TSO-8) and is a step that changes nothing. |
| TSO-7 | **Fork, join and channels.** Entering a weak `parallel` is enabled when the entering thread's buffer is empty, and every branch starts with an empty buffer. The `join` is enabled when every branch has finished and every branch's buffer is empty; flushes stay enabled until then. A `send` is enabled when the sender's buffer is empty. After `join all`, memory holds one value per location and no buffer exists. These conditions are `tso`'s gate on `fork`, `join` and `send` and its split and merge, ADR-027 SE-1 (e) and (f). Amended by ADR-027. |
| TSO-8 | **Instruction mapping.** An ordering becomes an x86 instruction by the standard C11-to-x86 mapping (Batty, Owens, Sarkar, Sewell and Weber): every load is `MOV`; a non-atomic, `relaxed` or `release` store is `MOV`; a `seq_cst` store is `XCHG` (TSO-5); every RMW is `LOCK`-prefixed (TSO-5); a `seq_cst` fence is `MFENCE` (TSO-6); other fences emit nothing. |

#### 3.3 Release-acquire with relaxed and `seq_cst` accesses (`ra`)

`ra` is RC11 as an operational model: the promise-free model of Kang et al.
for non-atomic, relaxed, release and acquire accesses and fences, with
`seq_cst` accesses ordered by the partial SC order of §15.

| ID | Rule |
| --- | --- |
| RA-1 | **State.** For each shared location `ℓ`, a **message sequence** in modification order (mo): each message holds a value, a **message view**, and an **attached** mark when an RMW wrote it directly after its predecessor. A **view** maps each shared location to one of its messages. Each thread `b` holds a current view `cur_b`, an acquire view `acq_b`, and a release view `rel_b(ℓ)` per location. One global view `S` serves `seq_cst` fences. The SC event graph of §15 and the race summary of §14 complete the component. Entering a top-level weak `parallel`, each location holds one message, its current value, with every view pointing at it. |
| RA-2 | **Load.** A load by `b` of `ℓ` reads any message `m` of `ℓ` at or after `cur_b(ℓ)` in mo. `cur_b(ℓ)` moves to `m`, and `acq_b` joins `m`'s view. An `acquire` or `seq_cst` load also joins `m`'s view into `cur_b`. A non-atomic load reads as a `relaxed` one. A transition identity names `m` by its **depth**: 0 for the mo-last message, counting back. |
| RA-3 | **Store.** A store by `b` to `ℓ` inserts a new message into `ℓ`'s sequence at any slot after `cur_b(ℓ)`, except directly after a message whose successor is attached. `cur_b(ℓ)` moves to it. A `release` or `seq_cst` store's message view is `cur_b`, and `rel_b(ℓ)` becomes `cur_b`. A `relaxed` or non-atomic store's message view is `rel_b(ℓ)` joined with the new message at `ℓ`. A transition identity names the slot by depth. |
| RA-4 | **RMW.** An RMW by `b` reads a message `m` at or after `cur_b(ℓ)` that has no attached successor, by RA-2 with the read part of its ordering, and inserts its message directly after `m`, attached, by RA-3 with the write part. Its message view also joins `m`'s view, so a release sequence continues through an RMW. Two RMWs never read one message. |
| RA-5 | **Fences.** An `acquire` fence sets `cur_b` to `cur_b` joined with `acq_b`. A `release` fence sets `rel_b(ℓ)` to `cur_b` for every `ℓ`. An `acq_rel` fence does both. A `seq_cst` fence does both, then sets `cur_b` and `S` to `cur_b` joined with `S`, and is an SC event of §15. |
| RA-6 | **Fork, join and channels.** Each branch starts with its parent's views. The `join` sets the parent's views to the join of its branches' views. A `send` carries the sender's `cur_b`, and its `receive` joins that view into the receiver's `cur` and `acq`: a channel synchronises as a release and an acquire. These are `ra`'s split, merge and channel rules through ADR-027 SE-1 (f), with no gate. Amended by ADR-027. |
| RA-7 | **Garbage collection.** After every step, every message of `ℓ` earlier in mo than the earliest message any view points to at `ℓ` (every thread's three views, every message view, and `S`) is removed, with PSC-4's summary of the SC events on it. No step can read a removed message: a load reads at or after its thread's current view. Removal is part of the canonical state and makes loops finite. After a top-level `join all` only the parent's views remain, and each location keeps one message. |
| RA-8 | **What it admits.** Coherence holds per location: views only move forward in mo. Program order with reads-from is acyclic, so load buffering is excluded, as in RC11. Independent reads of independent writes (IRIW) and 2+2W are admitted with release and acquire accesses, and excluded with `seq_cst` accesses, as in RC11. The intended correspondence is RC11; the litmus vectors below fix it. |

**Litmus outcomes.** "Allowed" means the weak outcome is reachable.

| Test | Weak outcome | `sc` | `tso` | `ra` |
| --- | --- | --- | --- | --- |
| SB, store buffering (§10) | both loads read 0 | forbidden | allowed | allowed |
| SB with `seq_cst` fences | both loads read 0 | forbidden | forbidden | forbidden |
| SB with `seq_cst` stores and loads | both loads read 0 | forbidden | forbidden | forbidden |
| MP, message passing, `release` store and `acquire` load of the flag | flag 1, data 0 | forbidden | forbidden | forbidden |
| MP with `relaxed` flag accesses | flag 1, data 0 | forbidden | forbidden | allowed |
| LB, load buffering, `relaxed` | both loads read the other's later store | forbidden | forbidden | forbidden |
| IRIW, `release` stores, `acquire` loads | the two readers disagree on the order of the writes | forbidden | forbidden | allowed |
| IRIW, all `seq_cst` | as above | forbidden | forbidden | forbidden |
| 2+2W, `release` stores | each location ends with its first-written value | forbidden | forbidden | allowed |
| CoRR, coherence of two reads | the second read returns an mo-earlier value | forbidden | forbidden | forbidden |

### 4. How this maps onto ADR-018's model

| ID | Rule |
| --- | --- |
| MS-1 | **Subject.** A subject with a weak `parallel` is a protocol subject (ADR-027 PS-1) whose state is (model state, control positions, memory component). The memory component is ADR-027 SE-1 (a), the member of the state key ADR-027 PS-8 names, in the typed canonical form of QSpec FR-181: buffers as sequences, message sequences in mo, views as maps from location to message position, the SC event graph and the race summary. It exists only while a weak `parallel` is active. Amended by ADR-027. |
| MS-2 | **Positions and observations.** Every step is a position (ADR-018 SM-3, ADR-027 PB-2), including `flush(b)`, which is observed with a `memory` anchor naming the step. Memory steps and fence steps are uncounted for interval operators (ADR-027 PB-3, SE-3; QSpec FR-431, FR-435-AC-6). A `holds` atom reads the model state, in which each shared location holds its **memory value**: under `tso`, memory; under `ra`, the value of its mo-last message. Branch-local locations hold their thread's value. Buffers, views, the SC event graph and the race summary are read by replay and counterexample rendering (MK-1), and atoms read values through ADR-027 SE-1 (g). Amended by ADR-027. |
| MS-3 | **Behaviours.** A behaviour is a maximal path of the protocol system (ADR-018 SM-3). Under `tso` it interleaves attempts with flushes; under `ra` each load and store step also names its message or slot (RA-2, RA-3), so the authored nondeterminism of reading and of mo placement is part of the successor relation (ADR-016 ND-1). |
| MS-4 | **Terminal states and deadlocks.** Under `tso`, `flush(b)` is enabled whenever a buffer is non-empty, so every terminal state has every buffer empty, and a fence, locked access, `send` or `join` waiting on a buffer is never a deadlock. A state whose only transitions are stores disabled by a memory bound is a boundary state (MB-3), never terminal. An SC access whose every message or slot choice would make the SC event graph cyclic (PSC-3) is disabled at that state, as RC11 excludes that execution. Otherwise terminal states and deadlocks read by ADR-018 SM-4 and DL-1 to DL-7 over the protocol system. |
| MS-5 | **State-space cost.** Under `tso`, a thread `b` that may store to `s_b` shared locations over value domains of size at most `d` has at most `Σ_{k=0}^{B} (s_b · d)^k` buffer contents with bound `B` (MB-2). Under `ra`, a location has at most `M` live messages (MB-2), each with a value and a message view of at most `M^L` choices over `L` shared locations, and a thread holds three views; the SC event graph has at most one live event per live message and thread (PSC-6), and the race summary is Boolean per location and thread (DR-3). Both multiply the SC state count by a factor exponential in the bound, the shared locations and the threads. §10 counts 16 states under `sc`, 38 under `tso` and 18 under `ra` for store buffering. |

### 5. Fairness

| ID | Rule |
| --- | --- |
| MF-1 | **Flush fairness under `tso`.** Every infinite-trace clause over a subject with a `tso` `parallel` carries, for each thread `b`, the constraint `fair weak each flush(b)` (ADR-018 FA-1, FA-3): no buffer stays non-empty forever while unflushed. The constraint belongs to the model, so it is not authored, it enters the clause's fairness set with a `memory` origin, and it is part of every counterexample and of replay (MK-2). On a lasso, every thread whose buffer is non-empty somewhere in the loop flushes in the loop. ADR-018 CX-5's walk takes it as a weak obligation. The memory constraints sit beside ADR-027 PA-3's scheduler constraints in every fairness set; they are ADR-027 SE-1 (h). Amended by ADR-027. |
| MF-2 | **Visibility fairness under `ra`.** Every infinite-trace clause over a subject with an `ra` `parallel` carries **visibility fairness**, after the memory fairness of Lahav et al.: no thread keeps reading a stale message of a location forever. On an SCC it is ADR-018 FA-4's form: for each thread `b` and location `ℓ` that `b` loads on some edge of the SCC, the SCC contains an edge on which `b` loads `ℓ` from its mo-last message. On a lasso, the loop contains such a load for every pair the loop loads. ADR-018 CX-5's walk takes each pair as a weak obligation, discharged by such an edge. The constraint sits beside ADR-027 PA-3's scheduler constraints. Amended by ADR-027. |
| MF-3 | **Machine closure.** Both constraints are machine-closed: every finite prefix extends to a behaviour that flushes every buffer, or reads every location's mo-last message, in turn. So, as ADR-018 §4 states for weak fairness, they never make a proof vacuous and never change a safety verdict (TP-1, TP-2, TP-3). |
| MF-4 | **Authored fairness.** An authored constraint names one of ADR-027 PA-1's targets, and memory steps are in none of its classes (ADR-027 PA-2), so authored constraints never require a memory step. Enabledness (ADR-018 FA-2) of a store disabled only by a memory bound reads it as enabled (MB-3), so a bound never makes an unfair lasso fair. Amended by ADR-027. |
| MF-5 | **Strong fairness.** The memory constraints are weak. ADR-019's filter treats them as weak rows beside any authored strong constraint. |

### 6. Exploration in EN-1

| ID | Rule |
| --- | --- |
| MB-1 | **The protocol system.** EN-1 explores the protocol system's successor relation on FR-101's canonical breadth-first engine (QSpec FR-181): ADR-027 ST-1 to ST-15, the fence step of a `fence` control, and the memory steps of §3, each with a typed canonical transition identity (ADR-027 TS-2). ADR-018's product, SCC phase, fairness filter, `ModelCheckLimits` and counterexample choice (ADR-018 §3, CX-5) apply unchanged. Amended by ADR-027. |
| MB-2 | **Memory bounds.** Two method bounds bound the memory component: `max_store_buffer`, the entries per thread's buffer under `tso`, and `max_messages`, the live messages per location after garbage collection under `ra`. Both join `ModelCheckRequest` beside `max_depth` as method bounds, like `max_depth`'s search horizon (ADR-018 §1): bounds on the explored state space that every result states, not resource limits. The request sets them, with no ceiling; a request that leaves one unset gets 4. They are not part of the model, the subject or the obligation identity. Every terminal record of an item over a subject with a weak `parallel` states, in its method, the bound used and whether the run reached it. |
| MB-3 | **Bound semantics.** A store that would exceed a bound is not expanded at that state; every other enabled step is. The run records the state as **bound-limited**. FA-2 enabledness reads the store as enabled. A state whose only enabled steps are bound-limited stores is a boundary state as ADR-021 SC-2 reads one: discovered, keyed, read by the monitor, never expanded, never stutter-extended, never a deadlock. |
| MB-4 | **Verdicts under a bound.** Bounded behaviours are behaviours of the unbounded model, so a counterexample found is real and settles V-4. A run that completes with no counterexample and no bound-limited state explored every behaviour of the unbounded model, by induction on the explored states, and settles V-1, with the bound stated as not reached. A run that completes with no counterexample and at least one bound-limited state settles V-6, `inconclusive`, cause `MemoryBoundReached{bound, states}`, with `bound` naming the request member that raises it (`max_store_buffer` or `max_messages`) and its value, and `states` the count of bound-limited states. The precedence among the under-approximation causes is ADR-021 RV-4's `ConstraintReached`, then `MemoryBoundReached`, then ADR-027 AE-4's `InstanceBoundReached`, then `BoundReached` (the completed `max_depth` search horizon). |
| MB-5 | **Other limits.** EN-1's other `ModelCheckLimits`, meters and cancellation settle as ADR-018 §3 and IV-6 state. |

The bound is a method bound because the unbounded model is infinite for any program
with a loop that stores: a buffer or a message sequence grows without limit.
Garbage collection (RA-7) keeps a loop finite when every thread keeps reading
recent messages.

### 7. Reductions

| ID | Rule |
| --- | --- |
| MR-1 | **Memory footprints.** A memory-model step has a static footprint over memory-component locations, fixed by §3, §14 and §15 and enforced by construction, so ADR-021 POR-2's enforcement holds for it with no author input. Under `tso`: a store by `b` reads and writes `buf(b)`; a load by `b` of `ℓ` reads `buf(b)` and `mem(ℓ)`; `flush(b)` reads and writes `buf(b)` and writes `mem(ℓ)` for every `ℓ` that `b` may store; a locked access by `b` reads `buf(b)` and reads and writes `mem(ℓ)`; a `seq_cst` fence reads `buf(b)`. Under `ra`: a load by `b` of `ℓ` reads `msgs(ℓ)` and reads and writes `view(b)`; a store or RMW reads and writes both; an `acquire`, `release` or `acq_rel` fence reads and writes `view(b)`; a `seq_cst` fence also reads and writes `S`; every `seq_cst` access and fence also reads and writes the SC event graph. Under both, an access to a location with a non-atomic access also reads and writes `race(ℓ)`. Branch-local footprints are ADR-021 POR-1's. Independence is ADR-021 POR-4 over these locations. Garbage collection removes only messages no step can read, and transition identities name messages by depth from the mo-last message, so independent steps commute with it. Memory footprints sit beside ADR-027 FT-1's protocol footprints, and a gate adds its memory locations to a step's read footprint. Amended by ADR-027. |
| MR-2 | **Visibility.** A step is visible (ADR-021 POR-5) when it writes a memory value an atom reads: under `tso`, a flush or locked access to such a location, never a plain store; under `ra`, a store or RMW to such a location. A step that writes a branch-local location an atom reads is visible. For the race-freedom item (§14), every access to a location with a non-atomic access is visible. |
| MR-3 | **Preservation.** The rows below, proposed for ADR-021 PT-2. |
| MR-4 | **Symmetry.** Permuting interchangeable objects (ADR-021 SYM-1) renames the locations inside buffers, message sequences, views, SC events and race summaries, and the canonicaliser re-encodes them (ADR-021 SYM-7); buffers keep program order and message sequences keep mo, neither of which reads identity order. Flush fairness names threads, which a permutation of objects fixes. Visibility fairness ranges over every location, and a permutation maps that family onto itself. Branches are control, so ADR-021 symmetry never permutes threads. |
| MR-5 | **Counterexamples.** A reduced search's counterexample is concretised by ADR-021 EI-6 with its memory component, and replays by MK-2. |

| Form, over a subject with a `tso` or `ra` `parallel` | Symmetry (admitted, ADR-021 SYM-5) | POR (ADR-021 POR-6, with MR-1 footprints) | State constraint |
| --- | --- | --- | --- |
| TP-1 reachable-state invariant | yes | yes | as ADR-021 PT-2 |
| TP-3 LTL safety, no interval operator | yes | yes | as ADR-021 PT-2 |
| TP-2, and TP-3 or TP-4 with an interval operator | yes | no: counts positions | as ADR-021 PT-2 |
| TP-4 liveness | yes, every authored constraint `whole` (MR-4 covers the memory constraints) | no: MF-1 and MF-2 make the fairness set non-empty | as ADR-021 PT-2 |
| Deadlock-freedom item | yes | yes (C0, C1) | as ADR-021 PT-2 |
| Race-freedom item (§14) | yes | yes, with MR-2's race visibility | as ADR-021 PT-2 |
| TP-5 refinement (ADR-020) | as ADR-021 PT-2 | no | as ADR-021 PT-2 |

### 8. The link to code

A proof under model `M` says: every behaviour of the subject under `M`, fair
under the clause's fairness set with `M`'s own constraints, satisfies the
claim. Code inherits the claim when every execution of the code, read through
the abstraction relation (QSpec FR-353, ADR-017 AR-2), is a behaviour of the
subject under `M`. That inclusion rests on the preconditions below. QSL states
them as preconditions, as ADR-021 TX-1 and TX-2 do; it never resolves a
`RustPath` (ADR-017 AR-2), so it checks none of them against Rust code. `M` is
the resolved model the verdict's obligation identity names (MM-5).

| ID | Rule |
| --- | --- |
| MX-1 | **Threads.** Each branch of a weak `parallel` runs as one thread, and its program order follows the branch's causal order. Entering the `parallel` and its `join` are thread spawn and join, which synchronise (Rust's `std::thread::scope` and `JoinHandle::join`). A channel's `send` and `receive` synchronise as a release and an acquire, as Rust's `std::sync::mpsc` does. |
| MX-2 | **Locations.** Each shared location with an atomic access binds (ADR-017 AR-2 `ObjectBinding.fields`) to a Rust atomic (`AtomicBool`, `AtomicU64` and the like) of a width that holds the field's domain. Each shared location whose accesses are all non-atomic binds to plain state, such as a field inside an `UnsafeCell`. A location with both atomic and non-atomic accesses binds to an atomic, and its non-atomic accesses are `unsafe` non-atomic reads and writes of it, as C11 permits. Each branch-local location binds to state only its own thread accesses. |
| MX-3 | **Accesses.** The function bound to an attempted operation (ADR-017 AR-2 `FrameBinding`) performs, on shared locations, exactly one access of the kind MA-2 gives it, to the bound field: a load; a store; or one RMW instruction (`compare_exchange`, `fetch_add` and the like, `swap`), never a load followed by a store. An atomic access's ordering is at least the attempt's declared ordering in C11's order: for loads `relaxed` < `acquire` < `seq_cst`; for stores `relaxed` < `release` < `seq_cst`; for RMWs `relaxed` < `acquire`, `release` < `acq_rel` < `seq_cst`. A non-atomic access in the model admits any access in code, atomic or not. A stronger ordering in code admits fewer executions, so the inclusion holds. A `fence` control binds to a `std::sync::atomic::fence` with at least its ordering. |
| MX-4 | **Per model.** *`sc`*: every attempted operation that touches a shared location runs atomically with respect to every other branch: one `SeqCst` access for a single-location operation (RC11 gives data-race-free programs whose atomics are all `SeqCst` only sequentially consistent behaviours), or a critical section under one lock that guards every shared location the operation touches. *`tso`*: the target is x86-64 with TSO-8's mapping, and every atomic shared load is at least `acquire` and every atomic shared store at least `release`, so that the compiler reorders only what TSO itself reorders (a store with a later load); non-atomic shared accesses are covered by the race-freedom item below. *`ra`*: the target has a compilation scheme sound for RC11 (x86-64, ARMv8, POWER). When a branch has a **load-buffering shape**, a `relaxed` or non-atomic load followed in program order by a store to another shared location with no `acquire` or `seq_cst` load or `acquire` fence between, the inclusion also needs a target whose compiled code excludes load buffering, such as x86-64. S3 reports each load-buffering shape as a warning, `memory.load-buffering-shape`, naming the load and the store. *Non-atomic accesses, under either weak model*: the subject's race-freedom item (§14) is proved. A race is undefined behaviour in Rust and C11, so a subject whose race-freedom item is refuted or unsettled transfers no claim to code. |
| MX-5 | **Per-function proofs.** Kani and Verus prove each bound function against its operation's contract sequentially (ADR-017 AR-5); the contract is memory-model-independent. With MX-3, each function's concurrent effect is the one access the model step stands for, and its contract fixes the values. The three together, the model proof under `M`, the per-function proofs and MX-1 to MX-4, give the code claim. |
| MX-6 | **Refutation strength.** A counterexample is a behaviour of the subject under `M`. Code whose orderings are stronger than declared (MX-3) may exclude it. |
| MX-7 | **Lowered-code checking.** A checker of lowered code advertises in its QSpec FR-290 provider manifest the memory models it decides, by the names of MM-1. Negotiation routes an item over a subject with a weak `parallel` only to a candidate advertising its resolved model, and never weakens the model to fit a candidate. The design of such checkers is later research (References). |

**Which back end checks which model.**

| Back end | `sc` | `tso` | `ra` |
| --- | --- | --- | --- |
| EN-1, explicit-state product (ADR-018 EN-1) | exact over the subject | exact up to `max_store_buffer` (MB-4) | exact up to `max_messages` (MB-4) |
| EN-2 and EN-3, SMT unrolling and induction (ADR-018 EN-2, EN-3) | the protocol system's transition relation over `k` steps | the same, with each buffer a bounded sequence of state variables | the same, with message sequences, views, the SC event graph and the race summary as bounded state variables |
| Kani and Verus, per-function contracts | memory-model-independent (MX-5) | as `sc` | as `sc` |
| Lowered-code checkers (MX-7) | by manifest | by manifest | by manifest |

EN-1's provider manifest advertises `sc`, `tso` and `ra`. EN-2 and EN-3
arrive with ADR-018's SMT path and encode the same operational state; an
axiomatic encoding is an alternative engine for that path.

### 9. Verdicts and counterexamples

| ID | Rule |
| --- | --- |
| MV-1 | **Verdicts.** Every verdict is one of ADR-018 V-1 to V-8. `InconclusiveCause` gains `MemoryBoundReached{bound, states}` (MB-4), category inconclusive. The `Unsupported` causes gain `WeakAccessShape{site, transition, shape}` (MA-4), settled in the pre-check, with `shape` one of `MultiLocation{locations}` and `JoinPolicy`. An item whose subject's resolved model no candidate advertises settles V-8, `unsupported-requested-capability`. A proof over a subject with a weak `parallel` is `Proved{basis: Exhaustive}`: the resolved model is in the obligation identity, and a run that reached no bound covered every behaviour (MB-4). An undefined claim evaluation settles by ADR-018 UE-1 to UE-6, `refuted` with cause `UndefinedEvaluation{where, cause}`; its finite prefix carries the memory component of each step and replays by MK-2, then ADR-018 UE-5. |
| MK-1 | **Content.** A counterexample is ADR-018 CX-2's `TemporalCounterexample`. A step's transition identity is an attempt, a fence, a control step or `flush(b)`; a load or store under `ra` names its message or slot by depth. Each step also carries its post-state's memory component in canonical form (MS-1): under `tso` every buffer; under `ra` every message sequence with its message views, every thread's views, `S` and the SC event graph; under both, the race summary when the subject has a non-atomic access. The memory component is data for the reader; the post-state digest already binds it. The fairness set lists MF-1 or MF-2's constraints with their `memory` origin. The counterexample names the resolved model of each outermost `parallel`. Memory-step identities sit beside ADR-027 TS-2's protocol transition identities, and the steps are ADR-027 PX-1's. Amended by ADR-027. |
| MK-2 | **Replay.** E9 replay (ADR-018 CX-3) re-executes each step through the protocol system under the resolved models: an attempt through `ModelSystem`'s FR-120 rule over the thread observation (MA-5), a memory step by §3. It recomputes the memory component and refuses with `stale_dependency`/`content-mismatch` when it differs from the carried one or the post-state digest differs, and with `invalid_runtime_input`/`invalid-value` when a step is not enabled or a named message or slot does not exist. A lasso is checked against the authored constraints and against MF-1 or MF-2; an unfair lasso refuses. Then SM-1 evaluates the formula, reading the lasso by ADR-018 SM-8, as CX-3 states. A race counterexample replays by DR-6 in place of the formula evaluation. |

### 10. Worked example: store buffering

All state counts below are hand enumeration; no QSL engine produced them.

**Model.** Object type `Cell` with `v: Int[0, 1]`, population `cells`,
universe `{x, y}`, both at 0. Operation `set()` with a receiver-scoped frame on
`v` (ADR-021 POR-3) and postcondition `self.v = 1`. Object type `Core` with
`r: Int[0, 2]`, population `cores`, universe `{c1, c2}`, both at 2, the
"not yet read" value. Operation `read(c: Cell)` with a receiver-scoped frame
on `r` and postcondition `self.r = pre(c.v)`.

**Protocol.** One `parallel`, `SB::Both`, with source default `sc` and two
branches, written as a litmus test:

| Branch `left` (role `c1`) | Branch `right` (role `c2`) |
| --- | --- |
| `Sx`: `set` on `x` (store `x := 1`) | `Sy`: `set` on `y` (store `y := 1`) |
| `Ly`: `read(y)` on `c1` (load `c1.r := y`) | `Lx`: `read(x)` on `c2` (load `c2.r := x`) |

Stores carry `ordering release` and loads `ordering acquire`. Footprints:
`(x, v)` is written by `left` and read by `right`, and `(y, v)` the reverse, so
both are shared; `(c1, r)` and `(c2, r)` are branch-local registers. MA-2
classifies `Sx` and `Sy` as atomic stores and `Ly` and `Lx` as atomic loads.
Every shared access is atomic, so no race-freedom item is added (DR-4).

**Claim.** `always holds(not (c1.r = 0 and c2.r = 0))`, a TP-1 invariant:
the two loads never both read 0.

**Three requests from one source.** A request with no `memory` member checks
the source default, `sc`. A request with `memory: [{parallel: "SB::Both",
model: "tso"}]` checks `tso`, and one selecting `ra` checks `ra` (MM-2). Each
is its own obligation (MM-5). Every request leaves the bounds unset, so each
runs with `max_store_buffer` and `max_messages` at 4.

**Counting.** Write each thread's local state as its program counter plus
what the model adds. `A`: before its store. `B`: stored, before its load.
`D0` and `D1`: done, having read 0 or 1. The joined state after `join all`
counts once per register outcome.

*Under `sc`* the local states are `A`, `B`, `D0` and `D1`. A thread reads 1
exactly when the other has stored, so `left` in `D1` needs `right` past `A`,
and `left` in `D0` needs `right` in `A` at the time of the load. That rules out
(`D1`, `A`), (`A`, `D1`) and (`D0`, `D0`) among the 16 pairs: **13** states in
the `parallel`, plus 3 joined states for outcomes (1, 1), (0, 1) and (1, 0):
**16**. No reachable state violates the claim.

*Under `tso`* a thread after its store has its store buffered (`b`) or
flushed (`m`), so the local states are `A`, `Bb`, `Bm`, `D0b`, `D0m`, `D1b` and
`D1m`: 7. A thread reads 1 exactly when the other's store is in memory, and a
flushed store stays flushed, so a thread in `D1b` or `D1m` needs the other
in `Bm`, `D0m` or `D1m`. Pairs with neither thread in a `D1` state: 5 × 5 =
25. Exactly one in a `D1` state: the other is `Bm` or `D0m`, 2 × 2 × 2 = 8.
Both in `D1`: each needs the other flushed, so (`D1m`, `D1m`), 1. That is
**34** states in the `parallel`. The `join` needs both threads done with empty
buffers: (`D0m`, `D0m`), (`D0m`, `D1m`), (`D1m`, `D0m`) and (`D1m`, `D1m`), 4
joined states: **38**. Four states in the `parallel` and one joined state
violate the claim.

*Under `ra`* each location has one message at 0 and, once stored, a second at
1, inserted after it (RA-3). The local states are `A`, `B`, `D0` and `D1`, as
under `sc`, since a thread's views are determined by its program counter and
what it read: `left` at `B` has `cur = (x: 1, y: 0)`, and reading `y`'s new
message gives `(x: 1, y: 1)`. The initial messages stay readable, because the
message view of `y`'s new message is `(x: 0, y: 1)` and pins `x`'s initial
message (RA-7). A thread in `D1` needs the other past `A`; `D0` needs nothing,
since the initial message is always readable. That rules out (`D1`, `A`) and
(`A`, `D1`): **14** states in the `parallel`, and 4 joined states: **18**. The
state (`D0`, `D0`) and its joined state violate the claim.

| Resolved model | States in the `parallel` | Joined states | Total | Verdict | Method |
| --- | --- | --- | --- | --- | --- |
| `sc` (source default) | 13 | 3 | 16 | `proved`, `Proved{basis: Exhaustive}` (V-1) | no memory bound applies |
| `tso` (request) | 34 | 4 | 38 | `refuted` (V-4) | `max_store_buffer` 4, not reached |
| `ra` (request) | 14 | 4 | 18 | `refuted` (V-4) | `max_messages` 4, not reached |

Every buffer holds at most one entry and every location at most two messages,
so neither bound is reached and each verdict is exact (MB-4).

**Counterexample under `tso`.** With transition identities ordered `left`
before `right` and attempts before flushes, the canonical counterexample is the
first violation in breadth-first order, at depth 4:

| Position | Step into it | Memory `x`, `y` | `left` buffer | `right` buffer | `c1.r`, `c2.r` |
| --- | --- | --- | --- | --- | --- |
| 0 | initial | 0, 0 | empty | empty | 2, 2 |
| 1 | `Sx` | 0, 0 | `[x := 1]` | empty | 2, 2 |
| 2 | `Ly`, reads `y` from memory | 0, 0 | `[x := 1]` | empty | 0, 2 |
| 3 | `Sy` | 0, 0 | `[x := 1]` | `[y := 1]` | 0, 2 |
| 4 | `Lx`, reads `x` from memory: `left`'s store is still buffered | 0, 0 | `[x := 1]` | `[y := 1]` | 0, 0 |

Replay (MK-2) re-executes the four steps under `tso`, recomputes both buffers
at each position, and evaluates the invariant false at position 4. Under `sc`
the same four steps give `c2.r = 1` at position 4, which is why `sc` proves
the claim.

**Counterexample under `ra`.** The same order, with message depths ascending:

| Position | Step into it | `x` messages | `y` messages | `cur` of `left`, `right` | `c1.r`, `c2.r` |
| --- | --- | --- | --- | --- | --- |
| 0 | initial | `x0` | `y0` | `(x0, y0)`, `(x0, y0)` | 2, 2 |
| 1 | `Sx`, release | `x0`, `x1{x1, y0}` | `y0` | `(x1, y0)`, `(x0, y0)` | 2, 2 |
| 2 | `Ly`, acquire, depth 0 (`y0`) | `x0`, `x1{x1, y0}` | `y0` | `(x1, y0)`, `(x0, y0)` | 0, 2 |
| 3 | `Sy`, release | `x0`, `x1{x1, y0}` | `y0`, `y1{x0, y1}` | `(x1, y0)`, `(x0, y1)` | 0, 2 |
| 4 | `Lx`, acquire, depth 1 (`x0`): `right`'s view of `x` is `x0` | `x0`, `x1{x1, y0}` | `y0`, `y1{x0, y1}` | `(x1, y0)`, `(x0, y1)` | 0, 0 |

Braces give a message's view. At position 4 the load could also read `x1`
(depth 0), which is the first successor and reaches `c2.r = 1`; depth 1 is the
second and violates the claim.

**With fences.** Adding `fence F ordering seq_cst;` between the store and the
load in each branch forbids the outcome under every model. Under `tso` the
fence waits for an empty buffer (TSO-6), so each load follows its own thread's
flush. Under `ra` the later of the two fences joins `S`, which holds the
earlier thread's store, into its thread's view (RA-5), so that thread's load
must read the other's new message. Both settle `proved` (V-1).

**With `seq_cst` accesses.** With every store and load `seq_cst` and no
fences, `tso` forbids the outcome because a `seq_cst` store is `XCHG` (TSO-5).
Under `ra` the SC event graph forbids it (PSC-3): if both loads read 0, each
load is fr-before the other thread's store, and with program order `Sx → Ly`
and `Sy → Lx` the graph has the cycle `Sx → Ly → Sy → Lx → Sx`, so the second
load's choice of the old message is disabled. Both settle `proved` (V-1).

**Message passing.** The MP shape (`data := 1; flag := 1` against `r1 :=
flag`, then `r2 := data` only in the case of a `choice` on the flag load's
binder that read 1, as MA-6 requires; claim `not (r1 = 1 and r2 = 0)`)
settles `proved` under
`sc` and `tso` with any orderings, `proved` under `ra` with a `release` flag
store and an `acquire` flag load, and `refuted` under `ra` with `relaxed` flag
accesses. With `data` non-atomic and the flag `release`/`acquire`, the
race-freedom item settles `proved` under `ra`: the data load runs only after
an `acquire` load that read the `release` store, so it happens after
`data := 1`. With a `relaxed` flag it settles `refuted` with a race
counterexample on `data` (§14).

### 11. Interactions with the sibling records

- **Strong fairness (ADR-019).** The memory constraints are weak rows in
  ADR-019's filter (MF-5). ADR-019 SR-1's enabled sets include memory steps,
  and a bound-limited store counts as enabled (MB-3).
- **Refinement (ADR-020).** A weak-memory concrete subject can refine an `sc`
  abstract model, which states that the concrete code behaves as the atomic
  specification. A memory step has no operation, so the step map has an
  explicit memory-step row for each memory step kind of the resolved model,
  `step memory <kind>` (ADR-027 PR-1, QSpec FR-432), and a concrete
  protocol with no row for a kind refuses `missing_declaration`/
  `missing-name`, naming the kind. A row's right side is written out:
  `-> any` where the intent is to pass when the mapped state is unchanged or
  some abstract transition matches, `stutter` or an abstract operation. A
  `fence` control has its own node row. A store under `tso` changes no memory value, so its mapped state is
  unchanged, and the flush that publishes it is the step that must match an
  abstract operation. Amended by ADR-027.
- **State-space reduction (ADR-021).** §7: memory footprints (MR-1), the
  proposed PT-2 rows (MR-3) and symmetry over objects (MR-4). The memory
  bounds and a state constraint are both under-approximations, ordered by
  MB-4's precedence.
- **Possible properties (ADR-022).** A litmus question, "can both loads read
  0?", is ADR-022's `possible holds(c1.r = 0 and c2.r = 0)`: its witness under
  `tso` is §10's prefix, and under `sc` it is refuted by the trap that is the
  whole 16-state reachable graph. A bound-limited state is an open node, as
  ADR-022 GM-6 reads a boundary state, so a trap that reaches one is not
  decisive. Memory fairness is machine-closed (MF-3), so ADR-022 GM-7's
  argument covers it.
- **Hyperproperties (ADR-023).** Self-composition takes two copies of the
  protocol system, each with its own memory component and resolved models.
  Each copy's memory constraints are scoped to its component, as ADR-023 HC-4
  scopes fairness.
- **Statistical properties (ADR-024).** ADR-024 PM-2's workload weights
  operations, and a memory step has none, so a probabilistic claim over a
  subject with a weak `parallel` settles V-8, `unsupported-requested-capability`,
  until a workload also weights memory steps.

### 12. Downstream impact and sequencing

**Stage DAG (ADR-011 §1).** No stage or edge is added. S3 gains the
`memory` clause, orderings and the `fence` control, the MM-3 and MA-3 to MA-6
refusals, the access classification in every `parallel`, and the
load-buffering warning (MX-4). The request writer adds the race-freedom item
(DR-4). The protocol system in `qsl-eval`'s `simulation` carries the memory
component. The `model_check` engine in `qsl-analyze` (ADR-029) gains the request selection (MM-2), the
memory bounds, the memory constraints in the fairness filter and the new
causes.

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: MM-1, MM-3, MM-6, MA-2 to MA-4 (symbolic part), the load-buffering warning. The request writer: the race-freedom item (DR-4). `qsl-eval`'s `simulation`: the memory component in the protocol system's state and key (MS-1), §3's rules, the SC event graph (§15), the race summary (§14), MA-5's thread observation, the depth-named transition identities. `model_check` (`qsl-analyze`, ADR-029): MM-2's resolution, the pre-check part of MA-4, MB-2 to MB-4 with the bound stated in the method, MF-1 and MF-2 in the filter and CX-5's walk, MR-1 footprints. `qsl-replay`: MK-1, MK-2, DR-5, DR-6, `MemoryBoundReached`, `WeakAccessShape`. The EN-1 manifest's memory models. |
| DS-2 | CG | The negotiation arm routes by resolved model and never weakens it (MX-7); obligation identity binds the resolved models (MM-5). |
| DS-3 | IR | With ADR-018's SMT path: the memory component in the transition-relation form. |
| DS-4 | Driver | None beyond ADR-018 DS-4. |

**Design notes for QSpec.** QSpec's shared grammar owns the `memory`
clause, the `ordering` member and the `fence` control; QSpec FR-331's request
owns the `memory` selection member and the resolved models in the obligation
identity. QSpec FR-052 and FR-228 state the interleaving meaning that `memory
sc` keeps; the weak models are amendments beside them. QSpec FR-181's state
key gains the memory member, and its transition identity the memory-step and
depth forms. QSpec FR-290's provider manifest carries the memory models a
candidate decides. The race-freedom item and its `Race` counterexample kind
sit beside the deadlock-freedom item (ADR-018 QS-12). The litmus table of §3
and §10's counts are the conformance vectors.

**Sequencing.**

1. **Specification.** This record, QSpec's half and the QSL compiler
   requirements FR-219 to FR-229.
2. **Prerequisites.** ADR-018 EN-1; ADR-027 and its `ProtocolSystem`, which explores `parallel` under `sc` with attempts through
   `ModelSystem`; ADR-021 POR-1 footprints and POR-3 receiver-scoped frame
   entries, which MA-1 and MA-2 read.
3. **`tso`.** Buffers, flushes, locked accesses, flush fairness, the buffer
   bound, request selection and the race-freedom item on EN-1. §10 under
   `sc` and `tso` is its first conformance vector.
4. **`ra`.** Messages, views, garbage collection, the SC event graph,
   visibility fairness and the message bound. The litmus table is its
   conformance corpus.
5. **SMT.** With ADR-018's EN-2 and EN-3.

### 13. Rulings on the draft's questions

The owner ruled on the six questions the draft left open, on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Whether the protocol transition system gets its own record first | **Its own record first.** ADR-020, ADR-021 and this record build on it | One foundation for protocol steps, with no divergent copies across three records. This record keeps its memory-model design and states its dependency on that record for protocol steps | Status; Context; MS-1; §12 sequencing step 2; References |
| RU-2 | Whether a request may choose the memory model | **Yes.** The source declares a default, the request may select another, and the resolved model enters the obligation identity | One source is checked under several models without edits, and the source still states what the author assumed. Binding the resolved model keeps a verdict under one model from joining a request under another | MM-1, MM-2, MM-5; MA-2, MA-4, MA-6; MK-1, MK-2; §10 |
| RU-3 | Whether `ra` refuses `seq_cst` accesses | **Add them.** RC11's partial SC order on `seq_cst` accesses and fences, after Lahav et al. | Rust code uses `SeqCst` widely. Reading it as acquire and release would admit more behaviours than RC11 gives it, and reading it as a fenced access fewer, so only the partial SC order is exact | §15, PSC-1 to PSC-6; RA-2, RA-3, RA-8; MA-3; MX-4 |
| RU-4 | Whether non-atomic shared locations are admitted | **Admitted, with a derived race-freedom item and its counterexample**, as ADR-018 DL-3 derives deadlock freedom | Real code keeps data in non-atomic locations guarded by synchronisation. A race is undefined behaviour, so it is reported once per subject as its own item, and every claim keeps its own truth | §14, DR-1 to DR-8; MA-2, MA-3; MX-2, MX-4 |
| RU-5 | The default memory bound | **4 when unset.** It is a method bound the request sets, and the result states the bound used and whether it was reached | A small default keeps runs short; the request raises it with no ceiling. Stating the bound and whether it was reached tells the reader how far a proof extends | MB-2, MB-4; §10 |
| RU-6 | Whether `ra` keeps relaxed accesses | **Keep them** | Relaxed accesses are common in Rust code. Load buffering stays excluded (RA-8) and is named as a transfer precondition (MX-4) | RA-2, RA-3, RA-8; MX-4 |

### 14. Data races and the race-freedom item

| ID | Rule |
| --- | --- |
| DR-1 | **Non-atomic accesses.** Under a weak model, a load or store with no ordering is non-atomic (MA-2). It explores as a `relaxed` access under `ra` (RA-2, RA-3) and as `MOV` under `tso` (TSO-8). |
| DR-2 | **Data race.** A data race is two accesses to one shared location by different threads, at least one a store or RMW and at least one non-atomic, where neither happens before the other. Happens-before is program order together with synchronisation: an `acquire` or `seq_cst` load, RMW or later `acquire` fence reading from a `release` or `seq_cst` store or its release sequence, as RA-2 to RA-5 join views; fork and `join` (RA-6, TSO-7); `send` and `receive`. This is RC11's definition. Under `tso` the same synchronisation is read over the store a load reads from, buffered or in memory. |
| DR-3 | **Race summary.** For each shared location `ℓ` with a non-atomic access, the memory component records the thread of `ℓ`'s last write and whether it was atomic, and for each thread whether its last read of `ℓ` was atomic. Each happens-before carrier holds Booleans: whether `ℓ`'s last write happens before it, and, for each thread `c`, whether `c`'s last read of `ℓ` happens before it. Under `ra` the carriers are every view of RA-1; under `tso` they are each thread's current and acquire summaries and each buffered store and memory value, which carries its storer's summary. A write to `ℓ` sets the write Boolean true for its own thread's carriers and false for every other carrier; a read of `ℓ` by `c` does the same for `c`'s read Boolean. A synchronisation joins carriers by Boolean or, where §3 joins views. Following FastTrack, comparing with the last write and each thread's last read decides every race. |
| DR-4 | **Derived item.** For each distinct subject among a request's items whose resolved model is weak and that has a non-atomic shared access, the request writer (ADR-013 O-20) adds one **race-freedom item**: a TP-1 invariant `always holds(not raced)`, where `raced` is derived and not authored. It is true at the post-state of a step that completes a data race: by DR-3, the step accesses `ℓ` non-atomically, or `ℓ`'s last write or a conflicting last read was non-atomic, and the conflicting last access is by another thread and does not happen before the step. Its obligation identity is the subject, the resolved models and the fixed item kind `race-freedom` (ADR-013 O-09). It has no opt-out. It settles as a TP-1 item: V-1 with no race, V-4 with a race counterexample, and V-5, V-6 (including `MemoryBoundReached`) and V-7 as for any TP-1 item. |
| DR-5 | **Counterexample.** A race counterexample is a CX-1 finite prefix from an initial state to the step that completes the race. `TemporalCounterexample`'s `kind` member (ADR-018 DL-4) gains `Race{location, earlier, later}`, naming the position of the earlier conflicting access and the position of the completing step. EN-1's canonical race counterexample is the canonical path to the first such step in FR-181 canonical breadth-first order, with `earlier` the last conflicting access the summary names. |
| DR-6 | **Replay.** MK-2 replays the prefix unchanged. Replay then computes happens-before over the replayed prefix from scratch, with vector clocks over the synchronisation of DR-2, independent of the race summary, and checks that the accesses at `earlier` and `later` are by different threads, touch `location`, include a write and a non-atomic access, and are unordered by happens-before. Agreement settles `reproduced-with-evaluated-witness` and the item `refuted`; disagreement settles `inconclusive`, `ReplayParity`. |
| DR-7 | **Claims read a racy behaviour as explored.** A race changes no authored claim's verdict: non-atomic accesses explore by DR-1, and the race-freedom item reports the race once. Transfer to code needs the item proved (MX-4). |
| DR-8 | **Engines.** EN-1 decides the race-freedom item in its first phase, evaluating `raced` at each step from the summary. EN-2 and EN-3 encode the summary as Boolean state variables. Under `sc` the item is not derived; MX-4's `sc` precondition covers concurrent access instead. |

### 15. RC11 `seq_cst` accesses under `ra`: the SC event graph

RC11 orders every `seq_cst` access and fence by `psc`, built from program
order, happens-before, modification order (mo) and from-reads (fr), and
requires `psc` to be acyclic (Lahav et al.). `ra` tracks the part of `psc`
that a future step can still close into a cycle.

| ID | Rule |
| --- | --- |
| PSC-1 | **State.** The memory component holds the **live SC events**, each a `seq_cst` access or fence with its thread, kind, location and message (the message it read or wrote), and a directed acyclic graph `P` over them: `e → x` when `e` precedes `x` in `psc`, closed under transitivity. For each location `ℓ` it also holds `F(ℓ)`, the live SC events that precede in `psc` every SC write to `ℓ` still to come. |
| PSC-2 | **Happens-before frontier.** Every view (RA-1) also records, for each thread, the latest live SC event of that thread that happens before the view, and whether that happens-before path leaves the event's thread through an event on another location, which RC11's `po|≠loc ; hb ; po|≠loc` clause reads. Frontiers are carried and joined wherever §3.3 carries and joins views. |
| PSC-3 | **A new SC event.** When thread `b` executes a `seq_cst` access or fence `x`, `P` gains an edge `e → x` for each live `e` that RC11's `psc` places before `x`, decided from the frontier (program order and happens-before), from locations (`hb|loc`), and from message positions (mo and fr); an edge `x → e` for each live SC write `e` that `x` precedes in mo (a store inserted before `e`'s message) or in fr (a load or RMW reading a message before `e`'s); and, for a write to `ℓ`, an edge from each member of `F(ℓ)`. A `seq_cst` fence's edges also follow RC11's fence clause, whose happens-before parts `S` already carries (RA-5). `x` is enabled with a given message or slot choice only when the resulting `P` is acyclic; each choice is a separate transition identity (RA-2, RA-3), so the check prunes exactly the choices RC11 forbids. |
| PSC-4 | **Collection.** When RA-7 removes a message, each live SC event on it leaves `P`. Its `psc` predecessors among the live events, and the event's own effect, are kept as follows: a removed write's predecessors and itself are summarised into `F(ℓ)` as predecessors of every later SC write to `ℓ`, since every later write is mo-after it; a removed SC read's predecessors join `F(ℓ)`, since every later write to `ℓ` is fr-after it; `P` is restricted to the remaining live events with its transitive closure kept. A thread's SC read of a message is replaced by its next SC read of the same message, which follows it in program order and so inherits its predecessors; a read takes no incoming edge from a later event (PSC-3), so nothing is lost. Every SC event is therefore live only while its message is, so `P` stays within the bound on messages. |
| PSC-5 | **Determinism.** `P`, `F` and the frontiers are part of the canonical state key (MS-1), encoded with events named by thread and message position, so equal histories that admit the same futures coalesce. |
| PSC-6 | **Cost.** `P` has at most one live event per live message and thread (PSC-4), so its size is bounded by the message bound, the shared locations and the thread count. The acyclicity check is one reachability query per new edge set. |

## Consequences

- An author states a default memory model on the `parallel`, and a request
  checks the same branches under `sc`, `tso` or `ra` without writing a store
  buffer or editing the source. Each model is its own obligation.
- A weak-memory proof from EN-1 is exact up to a memory bound and says so:
  every result states the bound used and whether it was reached, and a bound
  reached with no counterexample is `inconclusive`, never `proved`.
- `ra` is RC11: relaxed, release, acquire and `seq_cst` accesses and fences,
  with the partial SC order tracked over live events.
- Non-atomic locations are admitted, and every subject that has one reports
  its data races through one derived item with a replayable counterexample.
- Counterexamples show buffers, messages, views, SC events and race facts at
  every step and replay without the engine.
- The code link has named preconditions: one access per attempted operation,
  at least the declared ordering, race freedom for non-atomic accesses, and a
  per-model target condition. A proof under `ra` transfers to every
  RC11-sound target, a proof under `tso` to x86-64 only, and a proof under
  `sc` to code whose shared operations are `SeqCst` accesses or locked
  critical sections.
- Weak memory multiplies the state space by a factor exponential in the bound,
  the shared locations and the threads, so examples stay small; POR with
  memory footprints recovers much of it for safety.
- Weak memory rests on ADR-027's protocol transition system, which puts that
  record on its critical path.

## Amendments made with this record

Each amended cell or paragraph carries an "Amended by ADR-025" note.

- ADR-018 §1: the derived race-freedom item beside the deadlock-freedom item;
  "Scope" binds the resolved memory models; V-6 gains `MemoryBoundReached`;
  V-8 gains `WeakAccessShape`; the `InconclusiveCause` paragraph names
  `MemoryBoundReached`. §2 SM-3: memory steps are positions with a `memory`
  anchor and atoms read memory values. §4 FA-1: a weak model's own
  constraints join the fairness set. §5 CX-2: steps carry the memory
  component. §10 DL-2: a state whose only transitions are bound-limited stores
  is never a deadlock; DL-4: the counterexample `kind` gains `Race`.
- ADR-013 O-16: the two new causes and their categories.
- ADR-014 §1 B-5: `max_store_buffer` and `max_messages` join
  `ModelCheckRequest` beside `max_depth`, and the result states the bound
  used.
- ADR-011 §6.1: layer 3 `check` gains the access classification and weak
  memory refusals; `qsl-eval`'s `simulation` and the `model_check` engine in
  `qsl-analyze` (ADR-029) gain the memory
  component, bounds and fairness.
- ADR-017 AR-2: the bindings carry the MX-2 and MX-3 preconditions for a
  subject with a weak `parallel`.
- `spec/spec.md`: index row.

## Amendments to make on acceptance

- **ADR-021** PT-2: the rows of MR-3. POR-1: memory-model steps have MR-1's
  footprints. POR-9: the protocol system offers POR with MR-1's footprints.
- **ADR-019** §3: the memory constraints are weak rows (MF-5).
- **ADR-020** §2: each memory step kind has an explicit memory-step row,
  with `-> any` written out where that is the intent (ADR-027 PR-1, QSpec
  FR-432).
- **ADR-022** §6: the memory bound reads as a boundary state.
- **ADR-024** §1: a workload over a subject with a weak `parallel` weights
  memory steps, or the claim settles V-8.

## Alternatives Considered

- **A memory model only in source.** Rejected (RU-2). Checking one source
  under several models would need an edit per model, and each edit a new
  package.
- **A memory model only in the request.** Rejected. The source would be
  silent on what the author assumed. MM-1 keeps a default in source.
- **Store buffers written by hand in the model.** Rejected. It is what TLA+
  authors do today; it ties the model to one memory model and gives the code
  link no ordering to check.
- **An axiomatic model (execution graphs, RC11's consistency axioms) for
  EN-1.** Rejected for EN-1. EN-1 explores states and needs a successor
  relation; an axiomatic model checks whole executions, which is GenMC's
  method and suits lowered code. It stays available as an SMT encoding and as
  the oracle for RA-8.
- **The full promising semantics, with promises.** Rejected for now. Promises
  admit load buffering, as C11 and Rust permit on ARM and POWER, but each
  promise needs a certification search per step, which multiplies an already
  exponential state space. MX-4 instead names load buffering as a transfer
  precondition and reports each shape.
- **`seq_cst` accesses under `ra` read as their acquire and release parts.**
  Rejected. The result admits more behaviours than RC11 gives `SeqCst` code,
  so refutations would be spurious with no warning.
- **`seq_cst` accesses as an access between two `seq_cst` fences.** Rejected.
  It admits fewer behaviours than RC11, so a proof would not cover the code.
- **`seq_cst` accesses refused under `ra`.** Rejected (RU-3).
- **A complete execution graph for `psc`.** Rejected. Keeping every past SC
  event makes every loop infinite; PSC-4 keeps only events a future step can
  still order against.
- **Non-atomic shared accesses refused.** Rejected (RU-4).
- **A race as a conjunct of every claim.** Rejected, as ADR-018 rejects it
  for deadlocks: a claim would settle `refuted` where SM-1 says it holds. One
  derived item reports the race once (DR-4).
- **Full vector clocks in the state for race detection.** Rejected. Clock
  values grow without limit along a loop; DR-3's Booleans decide the same
  races and stay finite. Replay uses vector clocks over the finite prefix
  (DR-6), where growth does not matter.
- **Bounded buffers as a semantic choice, with a bound-limited store blocking
  as real hardware does.** Rejected. A proof under bound `B` would not cover
  `B + 1`; MB-4 keeps it `inconclusive`.
- **Multi-location operations as atomic blocks under weak models.** Rejected.
  No single atomic instruction realises one, so MX-3 could not hold; under
  `sc` they stay admitted with MX-4's lock precondition.
- **Thread symmetry.** Not in this record. ADR-021 permutes objects, not
  branches; interchangeable branches need a new PT-2 row.

## References

- Owning ticket: Linear QSL-372; the owner's rulings in §13 are recorded on
  it. QSpec half: QSpec FR-434 to FR-439, with fence and memory steps
  uncounted in QSpec FR-431 and memory-step rows in QSpec FR-432 (Linear
  STD-138). Built on ADR-018 (Linear QSL-366). Interacts with ADR-019 (Linear
  QSL-365), ADR-020 (Linear QSL-367), ADR-021 (Linear QSL-368), ADR-022
  (Linear QSL-369), ADR-023 (Linear QSL-370) and ADR-024 (Linear QSL-371).
- ADR-027, the protocol transition system (Linear QSL-396), which ADR-020,
  ADR-021 and this record build on (RU-1).
- Later research on checking lowered code with loom, Shuttle or GenMC:
  Linear RES-42 and RES-9. Related research: Linear RES-11.
- L. Lamport, "How to make a multiprocessor computer that correctly executes
  multiprocess programs", IEEE Transactions on Computers, 1979.
- P. Sewell, S. Sarkar, S. Owens, F. Zappa Nardelli and M. O. Myreen,
  "x86-TSO: a rigorous and usable programmer's model for x86
  multiprocessors", Communications of the ACM, 2010.
- S. Owens, S. Sarkar and P. Sewell, "A better x86 memory model: x86-TSO",
  TPHOLs 2009.
- M. Batty, S. Owens, S. Sarkar, P. Sewell and T. Weber, "Mathematizing C++
  concurrency", POPL 2011: the C11-to-x86 mapping.
- O. Lahav, V. Vafeiadis, J. Kang, C.-K. Hur and D. Dreyer, "Repairing
  sequential consistency in C/C++11", PLDI 2017: RC11 and the partial SC
  order.
- J. Kang, C.-K. Hur, O. Lahav, V. Vafeiadis and D. Dreyer, "A promising
  semantics for relaxed-memory concurrency", POPL 2017.
- M. Kokologiannakis and V. Vafeiadis, "GenMC: a model checker for weak
  memory models", CAV 2021.
- O. Lahav, E. Namakonov, J. Oberhauser, A. Podkopaev and V. Vafeiadis,
  "Making weak memory models fair", OOPSLA 2021.
- C. Flanagan and S. N. Freund, "FastTrack: efficient and precise dynamic
  race detection", PLDI 2009.
- M. F. Atig, A. Bouajjani, S. Burckhardt and M. Musuvathi, "On the
  verification problem for weak memory models", POPL 2010.

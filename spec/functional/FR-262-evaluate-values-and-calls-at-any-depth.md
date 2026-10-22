---
id: FR-262
title: "Evaluate, key and handle values and calls at any depth"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: references
---
# FR-262: Evaluate, key and handle values and calls at any depth

## Description

The S6a evaluator and simulation SHALL evaluate calls of any depth and
handle values and value types of any depth, bounded by the execution budget
under `quire.value.accounting/v1` (ADR-030 D-4.7). QSpec FR-146 defines
execution fuel as the `work_units` limit, with one `function.call` charge per
call and no call-depth counter; this requirement carries it through QSL's
evaluator.

The QSL-owned shared evaluation-core interface SHALL own resumable application
state, the scheduler step and execution fuel for interpreted checked expressions
and registered generated host bodies. This specifies a required interface;
leaf extraction and host registration are not claimed to be implemented.

## Behavior

1. **Explicit task stack.** Expression evaluation and calls SHALL run on an
   explicit heap task stack, so no evaluation step uses native recursion
   whose depth grows with the input.
2. **Fuel.** Each call SHALL charge `function.call` under
   `quire.value.accounting/v1`. When a charge would exceed `work_units`, the
   evaluator SHALL return `incomplete { limit_kind: work_units, ... }` at the
   denied charge, naming the counter, which is its setting.
3. **Iterative value types.** `quire_exact::ValueType` SHALL clone, compare,
   hash, format for debug and drop over an explicit heap stack, as
   `quire_exact::Value` does, including on `no_std` targets.
4. **State keys.** A simulation state key and a transition identity SHALL be
   written into `quire-canonical`'s canonical writer from an explicit stack
   through an event-source bound on `TransitionSystem::Key` and
   `TransitionId`. `quire_exact::Value` SHALL implement that bound
   iteratively, and a key of fixed shape SHALL implement it through its
   `FixedShape` serde path (FR-259).
5. **Exploration limits.** Simulation SHALL keep its exploration limits
   (FR-101), which bound states, transitions and path length.

### Ownership and admission

The shared interface belongs to QSL, in a leaf crate in its own repository:
`quire-eval` in `agent-ix/quire-eval`, at ADR-011 layer E
([FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md),
which also places every type the interface names).
It SHALL use `core` and `alloc` and forbid unsafe code. QSL's existing std
evaluator remains the semantic reference behind that interface. The interface
deliverable (IR-583) and checked-term separation (IR-586) are independent of
each other; the no_std evaluator implementation behind the interface is a
later milestone (IR-590), after both and the other no_std prerequisites. Accepting the interface SHALL
NOT imply availability of a no_std evaluator. RT and CG SHALL consume this interface, without
depending on evaluator internals.

For this extraction, [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)'s
layer 5 remains the S6a family, simulation and checked-package adapter owner.
The shared core sits below it, without dependencies on QSL's checker, package,
model-intake, routing, replay or backend crates. Separating constructor-private
checked-term types and runtime model-admission dependencies from the checker
precedes extraction of the no_std evaluator implementation, rather than
definition of the interface. That separation SHALL preserve checking authority, source
locations, declaration identities, model correspondence and proof closure.
[FR-400](FR-400-separate-checked-terms-from-the-checker.md) states that
separation: the `quire-checked` crate at ADR-011 layer C, the types it holds,
the source-location representation, the checker's sole construction
authority and the checks that prove each.

When replacing RT's ported function-application implementation, RT SHALL
consume an implementation of this interface that actually runs under its
`no_std + alloc` target and supplies the required evaluation, checked-input
and generated-body surface. This is the IR-590 implementation deliverable,
not merely IR-583's interface crate with the std reference adapter. The complete
RT migration SHALL wait for that implementation and its consumer acceptance
checks; adopting the interface alone cannot complete deletion of RT's evaluator
copy. A std consumer can adopt IR-583's seam earlier. The later default-evaluator
swap is a separate milestone; it is not required for RT to consume an already
accepted no_std implementation. This allocation grants no interim-copy exception
and introduces no compatibility shim or second lowering.

When registering a host function, the core SHALL bind a resumable body factory
to an exact callable identity and signature already admitted in the selected
checked closure. Registration SHALL NOT mint checked authority, replace its
proofs, or resolve calls by ambient name lookup. Capability negotiation and
unsupported preparation remain before application under
[FR-294](FR-294-run-every-execution-backend-behind-one-seam.md).

The host-body registry SHALL belong to one exact checked closure. Its lifecycle
is open for registration, then permanently sealed by its first successful
Begin application. A failed Begin SHALL leave an open registry open. When
Begin succeeds, the core SHALL seal the registry before any factory invocation
or application charge and bind the session to that immutable registry. Later
sessions may use the same sealed registry; termination or dropping a session
SHALL NOT reopen it. Registration and Begin SHALL have exclusive access while
they decide admission, so a registration cannot race the sealing boundary.

Registration SHALL return one typed internal admission result: `Accepted`,
`RegistrySealed`, `CallableNotAdmitted`, `SignatureMismatch`, or
`DuplicateBinding`. These are in-process registration results, not new public
runtime refusal codes, kernel outcomes or JSON wire causes. The core SHALL
decide them in this order: a sealed registry returns `RegistrySealed`; otherwise
an identity absent from that registry's checked callable closure returns
`CallableNotAdmitted`; otherwise a signature unequal to its admitted signature
returns `SignatureMismatch`; otherwise an identity already registered returns
`DuplicateBinding`; otherwise the binding is stored and returns `Accepted`.
When both sealed and duplicate apply, the result SHALL be `RegistrySealed`.

A second binding SHALL be refused even when it supplies the same factory or
an equally valid factory; registration is neither replacement nor idempotent
acceptance. A refused registration SHALL leave every binding unchanged.
Registration SHALL neither invoke a factory nor participate in the application
meter. Once sealed, the registry SHALL retain its bindings for every session
that uses it, including nested applications not yet reached. Selecting a new
set of factories requires a fresh registry and fresh sessions, rather than
changing an existing session's binding view.

### Operational interface

These are typed in-process operations, not a wire format or second lowering.
Their semantic inputs and results are normative; concrete Rust item names
belong to the interface implementation.

| Operation | Inputs | Result and responsibility |
| --- | --- | --- |
| Register host body | Open or sealed checked-closure-bound registry; admitted callable identity and signature; resumable body factory | Typed internal admission result under the ordered policy above; accepted factory supplies fresh invocation state for each call |
| Begin application | The registry, which holds its one selected checked closure; exact callable and completed arguments, or checked expression and parameters; admitted object environment; one `Meter`, whose installed `Cancel` handle, if it has one, is the session's handle | Pre-charge input failure, or one opaque session bound to these inputs and the sealed registry |
| Step session | Exclusive access to that live session | Still runnable, or terminal `Evaluation` / `CallFailure` under the existing S6a contract |
| Resume body | Invocation state; initial arguments or the matching child's evaluation; scoped access to the session meter | Continue with saved state, request a child application, or return an evaluation |

Begin SHALL take no `Cancel` of its own. Cancellation reaches a session
only through its meter's installed handle, which is the handle the caller
cancels; a session whose meter has no installed handle cannot be cancelled.
Begin's argument admission, for a callable's arguments and for a checked
expression's parameter values, SHALL take the session's meter. The
supplied-value run it admits arguments with
([FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md))
SHALL poll the meter's installed handle at FR-277's poll points; those
points are the only places admission observes cancellation. Admission
charges the meter nothing. A handle that is already cancelled, or is
cancelled at one of those points, ends Begin with the pre-charge
`CallFailure::Cancelled` carrying the handle's cause, with no session and no
`function.call` charge. When the meter has no installed handle, the run
Begin builds for itself, used when the caller passes none, polls a handle no
caller holds; a run the caller passes keeps its own `Cancel` (FR-277).

A caller MAY pass its own supplied-value run, and SHALL build it over the
handle it installed in the meter. When the run's `Cancel` is a different
handle, admission SHALL poll both at each of FR-277's poll points, the run's
handle first (when the meter has no installed handle, the run's handle
alone), and the first handle found cancelled ends Begin with
`CallFailure::Cancelled` carrying that handle's cause.

A session SHALL retain its task stack, call frames, lexical bindings,
package/import context, source locations, losses and accounting across steps.
A session SHALL NOT be copied, rebound to another checked closure, environment
or meter, or resumed after a terminal result. Synchronous `call` and `evaluate`
SHALL drive this same session until terminal with their existing result and
input-refusal contracts.

When requesting a nested application, a body SHALL return the exact admitted
callee, completed arguments, call-site location and saved caller continuation
to the scheduler. The scheduler SHALL validate the request, charge the call
and push the callee frame without native recursion. Validation checks that the
callee is admitted in the session's checked closure, that the argument count
equals its admitted signature's parameter count, and that each argument is a
member of its parameter's `ValueType`. For a `Population<T>[N]` parameter,
membership is decided as Begin decides it: the environment contract resolves
the argument's `PopulationId`, and the resolved binding's declared maximum
must equal the parameter's
([FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md) argument
admission check 2). For every other parameter, membership is decided in the
type environment of the session's checked closure. Neither check uses a
supplied-value run: each charges neither the meter nor a supplied-value
budget, and polls no `Cancel` handle. Validation therefore never charges the meter and never observes
cancellation. A request that fails any of
these checks, including a `Population` argument whose `PopulationId` the
environment does not resolve or whose binding's declared maximum differs, is a
broken invariant of the requesting body, not caller input:
the session SHALL terminate with `CallFailure::Fault` carrying an
`InternalFault` whose stage is `quire-eval` and whose invariant is
`child-request-not-admitted`, category `internal failure`
([FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md)). It
terminates before the child's `function.call` charge, invokes no callee body,
does not resume the requesting body, and keeps earlier spend. Such a request
SHALL NOT end the session with `CallFailure::Input`. A membership check that
cannot decide for want of storage SHALL terminate the session with
`CallFailure::Storage`, whose `parameter` is the index of the callee's
parameter and whose failure is the type environment's storage failure, before
the child's `function.call` charge and keeping earlier spend. No other failure
arises from validation. While the child is active,
the caller SHALL remain suspended. When the child completes, the scheduler
SHALL deliver its evaluation to the matching continuation exactly once.
Non-completed outcomes SHALL propagate under existing expression rules without
inventing a value. Generated closures remain a first-class lowering, with
invocation state retaining the program position and locals after each child.

A step SHALL advance one finite saved body/task transition and commit its state
before returning control. Still runnable is a scheduling boundary, not
`Incomplete`, a value, or a charge point. A body SHALL hand every governed
nested application to the scheduler rather than recursively entering `call`,
`evaluate`, or synchronous `Frame::call` to obtain a child's value. Each
transition's local host work must terminate; arbitrary host recursion or
blocking I/O lies outside the checked evaluation contract. No time-slice or
latency guarantee is allocated here.

### Fuel and terminal boundaries

All frames and bodies SHALL use the same existing `quire.value.accounting/v1`
meter, including its non-work counters. Fuel is its `work_units` limit, not a
second budget or scheduler-step count. Source arguments SHALL evaluate left to
right before completed-input validation. Validation SHALL precede
`function.call`, which SHALL precede binding and body execution. A checked
expression root SHALL incur no call charge unless it reaches an application.

A continuation SHALL neither replay completed work nor recharge an admitted
call. Saving state, yielding, pushing/popping a frame and delivering a child
SHALL add no accounting charge. Kernel operations SHALL keep their existing
charge points and order. When a charge is denied, the core SHALL terminate
with the existing located incomplete result at that charge, leaving the denied
charge's counters unchanged and retaining earlier successful spend. A fresh
invocation is required after raising limits; static totality stays unchanged.

The meter SHALL poll its installed handle at every charge, as
[FR-276](FR-276-cancel-a-lifecycle-operation.md) requires, and the core SHALL
observe cancellation only at a meter charge or, during Begin's argument
admission, at one of FR-277's poll points. A charge the handle denies is a denied charge
like any other: the session SHALL terminate with that charge's located
incomplete result, with `limit_kind: work_units` and `limit` equal to
`consumed`, the denied charge unrecorded and earlier spend retained, whichever
frame, body or `function.call` made the charge. The handle's `tripped()`
then reports the cause. The core SHALL NOT turn that result into
`CallFailure::Cancelled`; it is the result the reference evaluator's `call`
and `evaluate` return for the same charge. The lifecycle operation that holds
the handle reads `tripped()` and returns its own cancelled failure, as FR-276
states. A cancellation made between two steps is observed at the next charge;
a transition that makes no charge runs to its end. The core returns
`CallFailure::Cancelled` only as a pre-charge Begin failure from the
supplied-value run ([FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md)).
When the session terminates with completion, refusal, undefinedness,
incomplete, fault or storage failure, the core SHALL release suspended frames without executing
continuations.
Locations, loss records and outcome propagation SHALL match the reference
evaluator; a scheduling yield is never an evaluation result.

[ADR-029](../decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md)
EB-1's `ExecutionBackend` / `PreparedEntries` remains the outer seam. The shared
body/scheduler interface sits inside execution; it SHALL NOT select backends
or change prepared-entry unsupported dispositions. When an in-process backend
invokes a registered host body, it SHALL save that body's continuation at the
end of the step and use the same session for child handoffs. A host invocation
already performed SHALL NOT be lost or repeated across a step. AOT transport
and JIT preparation retain their existing contracts.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-262-AC-1 | On a thread with a 512 KiB stack, the recursive function `function count using v(n: Int[0, 1000000]): Integer pure decreases(n) { if n = 0 then 0 else count(n - 1) + 1 }`, called with `100000` under a `work_units` budget sized for it, completes with `100000`. With `work_units` one below the run's measured spend, the same call returns `incomplete { limit_kind: work_units }` at the denied charge, and completes once `work_units` is raised to the measured spend. | Test |
| FR-262-AC-2 | On a thread with a 512 KiB stack, a recursive list value 100,000 long (`record List { head: Int[0, 9]; tail: List?; }`), built by a recursive function, evaluates; it keys as simulation state, and its state-key bytes equal the RFC 8785 text of its canonical JSON form; it compares equal to its clone, and it and its clone drop. A `ValueType` of 100,000 nested `Option`s around `Boolean` clones, compares equal to its clone, hashes equal to its clone, formats for debug and drops on the same thread. | Test |
| FR-262-AC-3 | The core has no checker/package dependency or RT/CG access to its internals, and registration cannot create checked authority or substitute another closure. In an open registry, registering valid factory A for admitted identity F returns `Accepted`; registering valid factory B for the same identity/signature, or A again, returns `DuplicateBinding` without changing A's binding. A failed Begin leaves registration open; successful Begin seals it. Registering B for F after that Begin returns `RegistrySealed` (before duplicate), as does registering a different admitted identity. That session and later sessions keep A for F, and session termination does not reopen registration. Registration invokes neither factory and makes no application charge. | Test |
| FR-262-AC-4 | A generated body calls a second generated body that calls an interpreted function. Stepping between every handoff and return produces the reference result and charge sequence, resumes each caller once, preserves locals and call-site locations, and never recursively invokes the evaluator. | Test |
| FR-262-AC-5 | A source call with two argument applications charges them left to right before its own call charge and body work. Steps add no charge; a standalone literal charges no `function.call`; rejected root inputs cause no charge or body invocation. | Test |
| FR-262-AC-6 | For AC-4's mixed-body chain let `w` be reference work spend. Limit `w` completes; `w - 1` returns the reference located incomplete result and denied charge, keeps earlier spend and leaves the denied charge unrecorded. No subsequent step can run a suspended body; a fresh invocation at `w` completes with static totality unchanged. | Test |
| FR-262-AC-7 | A body saves a child request, and the meter's handle is cancelled with `Requested` before the next step. That step returns the located incomplete result at the child's `function.call` charge, with `limit_kind: work_units`, `limit` equal to `consumed` and the request's call-site location. The charge is not recorded, no child body is invoked, no later charge is made, and `tripped()` returns `Requested`. In two further runs, a child returns a refusal in one and an undefined outcome in the other, to a body that returns its child's outcome unchanged. In each, the requesting body is resumed exactly once with that outcome, and it returns the outcome with the reference's location and losses. The body is not resumed again, and no transition follows its return. Dropping a runnable session executes no continuation. | Test |
| FR-262-AC-8 | A body performs charged work, saves a local, requests a child and continues with its returned value. Each body transition and child occurs exactly once across steps; prepared-entry execution matches the synchronous reference, including a kernel loss record, and unsupported preparation stays a pre-application disposition. | Test |
| FR-262-AC-9 | An interface crate with only a std reference adapter does not satisfy RT's complete evaluator migration. That migration consumes the shared implementation with std disabled on RT's governed no_std target, supplies checked-input and generated-body evaluation through this interface, and leaves no RT-local evaluator copy or compatibility re-export. | Inspection |
| FR-262-AC-10 | Begin takes no `Cancel` argument. With a meter whose installed handle is cancelled with `Deadline` at the first kernel charge a registered body makes, the session terminates with that charge's located incomplete result (`limit_kind: work_units`, `limit` equal to `consumed`), not `CallFailure::Cancelled`; `tripped()` returns `Deadline`. With the handle cancelled at a `function.call` charge, the result has the same form at that charge. When a caller cancels between two steps and the next transition makes no charge, that transition runs and the session terminates at the first charge after it. | Test |
| FR-262-AC-11 | A registered body requests, in five runs: a `NodeKey` that no function of the closure has; the admitted `f(x: Int[0, 9]): Boolean` with two arguments; `f` with the argument `10`; a function `h(p: Population<T>[3]): Boolean` with a `Population` value whose `PopulationId` the environment does not resolve; and `h` with a `Population` value the environment resolves to a binding of declared maximum `4`. Each run terminates with `CallFailure::Fault` whose `InternalFault` has stage `quire-eval` and invariant `child-request-not-admitted`, category `internal failure`. The run makes no `function.call` charge for the child, invokes no callee body, does not resume the requesting body, and keeps the charges made before the request. In a sixth run, `h` requested with a `Population` value the environment resolves to a binding of declared maximum `3` is applied, with one `function.call` charge for it. Begin of `f` with two arguments returns `CallFailure::Input(InputRefusal::Arity)`, and Begin of `f` with `10` returns `CallFailure::Input` from argument admission, each with no charge. | Test |
| FR-262-AC-12 | Begin of the admitted `g(x: Int[0, 9], y: Int[0, 9]): Boolean` with arguments `3` and `4`, and Begin of a checked expression with those two parameter values, each return `CallFailure::Cancelled` with no session, all of the meter's consumed counters zero and no `function.call` charge recorded, in four runs: the meter's installed handle cancelled with `Requested` before Begin (cause `Requested`); the meter's handle tripping with `Deadline` at the supplied-value run's first poll point for the second argument (cause `Deadline`); a caller-supplied run over a different handle cancelled with `Requested`, the meter's handle not cancelled (cause `Requested`); and that run's handle not cancelled, the meter's handle cancelled with `Deadline` (cause `Deadline`). With no handle cancelled, both Begins return a session; with a meter that has no installed handle and the caller's run over an uncancelled handle, both Begins return a session. With a meter that has no installed handle and a caller-supplied run whose handle is cancelled with `Requested`, both Begins return `CallFailure::Cancelled` with cause `Requested`, no session and zero consumption. | Test |

## Dependencies

- Shared interface ownership: QSL-682 then IR-583; checked-term extraction:
  IR-586 under FR-400, independent of the interface; no_std evaluator
  implementation: IR-590 after both and its other prerequisites. RT consumes this contract in
  `ix://agent-ix/quire-contract-runtime/FR-273` (IR-512); its resumable-body
  implementation is downstream work.

- The `std` feature, target, toolchain and `make build-no-std` gate for the
  QSL crates on the evaluator path:
  [NFR-015](../non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md).
- The `no_std` implementation in `quire-eval`, its environment contract and
  its rerun of AC-1 to AC-8, AC-10 and AC-11:
  [FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md).
- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.7 and D-6.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  defines exploration and its state keys.
- [FR-259](FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
  defines the event API and the fixed-shape path.

## References

- QSpec FR-146 (execution fuel is `work_units`; call nesting depth is not a
  counter).
- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.

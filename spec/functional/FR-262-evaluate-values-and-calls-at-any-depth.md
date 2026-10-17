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

The shared interface belongs to QSL, in a leaf crate in its own repository.
It SHALL use `core` and `alloc` and forbid unsafe code. QSL's existing std
evaluator remains the semantic reference behind that interface. The interface
deliverable (IR-583) precedes checked-term separation (IR-586); the no_std
evaluator implementation behind it is a later milestone (IR-590), after that
separation and the other no_std prerequisites. Accepting the interface SHALL
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
| Begin application | Selected checked closure and its registry; exact callable and completed arguments, or checked expression and parameters; admitted object environment; one `Meter`; `Cancel` | Pre-charge input failure, or one opaque session bound to these inputs and the sealed registry |
| Step session | Exclusive access to that live session | Still runnable, or terminal `Evaluation` / `CallFailure` under the existing S6a contract |
| Resume body | Invocation state; initial arguments or the matching child's evaluation; scoped access to the session meter | Continue with saved state, request a child application, or return an evaluation |

A session SHALL retain its task stack, call frames, lexical bindings,
package/import context, source locations, losses and accounting across steps.
A session SHALL NOT be copied, rebound to another checked closure, environment
or meter, or resumed after a terminal result. Synchronous `call` and `evaluate`
SHALL drive this same session until terminal with their existing result and
input-refusal contracts.

When requesting a nested application, a body SHALL return the exact admitted
callee, completed arguments, call-site location and saved caller continuation
to the scheduler. The scheduler SHALL validate the request, charge the call
and push the callee frame without native recursion. While the child is active,
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

The core SHALL poll `Cancel` at every charge, as
[FR-276](FR-276-cancel-a-lifecycle-operation.md) requires, as well as before
each scheduler transition. When cancellation is observed before the next transition, the core SHALL
terminate with the existing `CallFailure::Cancelled`, without executing another
body, child or charge. A transition already in progress SHALL save its committed
state at its end before control returns to the caller; cancellation SHALL NOT
replay it. When the session terminates with completion, refusal, undefinedness,
incomplete, fault or cancellation, the core SHALL release suspended frames
without executing continuations.
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
| FR-262-AC-7 | Cancelling after a child request is saved, before the next transition, returns `CallFailure::Cancelled` with no further charge or body execution. In a direct-return caller, a child refusal or undefined outcome propagates with reference location/loss behavior and runs no subsequent body work; dropping the session executes no continuation. | Test |
| FR-262-AC-8 | A body performs charged work, saves a local, requests a child and continues with its returned value. Each body transition and child occurs exactly once across steps; prepared-entry execution matches the synchronous reference, including a kernel loss record, and unsupported preparation stays a pre-application disposition. | Test |
| FR-262-AC-9 | An interface crate with only a std reference adapter does not satisfy RT's complete evaluator migration. That migration consumes the shared implementation with std disabled on RT's governed no_std target, supplies checked-input and generated-body evaluation through this interface, and leaves no RT-local evaluator copy or compatibility re-export. | Inspection |

## Dependencies

- Shared interface ownership: QSL-682 then IR-583; checked-term extraction:
  IR-586 after the interface; no_std evaluator implementation: IR-590 after
  its prerequisites. RT consumes this contract in
  `ix://agent-ix/quire-contract-runtime/FR-273` (IR-512); its resumable-body
  implementation is downstream work.

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

---
id: AD-001
title: "Shared evaluation-core interface and staged evaluator replacement"
type: ArchitectureDescription
status: proposed
owner: qsl-maintainers
system: QSL shared evaluation-core leaf crate, its std reference adapter and the staged no_std evaluator
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: references
---
# AD-001: Shared evaluation-core interface and staged evaluator replacement

## System Boundary

This description records the implementation boundary for the QSL-owned shared
evaluation core and orders its transition from the current std evaluator to a
no_std + alloc implementation. The normative behavior is FR-262; this record
maps that behavior to the current QSL source and the IR-583 / IR-584 slice plan.
The interface is shared across QSL, RT and CG. It does not make any consumer a
second semantic authority.

The shared leaf owns the checked-expression execution interface, resumable
application state, scheduler step, work-fuel accounting, and host-body
registration/admission. It depends only on the checked-term and semantic
vocabulary it needs, `core`, `alloc`, and shared kernel crates. It has no
dependency on QSL's checker, package linker, model intake, routing, replay, or
backend selection. Every newly created crate in this path forbids unsafe code.

QSL's current std evaluator remains the reference implementation behind this
interface during the migration. IR-583 delivers the shared interface and std
reference adapter. It does not claim that the evaluator itself is no_std. The
no_std + alloc implementation is IR-590, after S1-S5. IR-269 remains a direct
blocker of S1 and therefore an indirect predecessor of S6. During the bounded dual-implementation period, differential tests
compare both implementations on the same checked inputs. IR-593 ends that
period by selecting the no_std implementation as the default and retiring or
demoting the std-only path.

## Views

The system is viewed through the current source boundary, the interface flow,
execution limits, staged ownership, and downstream consumer acceptance.

### Current source boundary

The source boundary was rechecked against QSL main on 2026-10-22.
`CheckedGraph` and `CheckedExpression` are checker outputs in
`qsl-semantics/src/check/mod.rs`; they are not yet an independent evaluator
input crate. `CheckedPackage` is owned by `qsl-package`. The public evaluator
trait `CheckedPackageEvaluation` and its `call` / `evaluate` implementation are
in `qsl-eval/src/value/expression/mod.rs`; the implementation constructs an
internal `Machine` over the checked graph, object environment, meter and
dispatch tables. `evaluate_closed` is an additional internal-to-QSL entry
point. There is no host-function registration API in the current QSL
evaluator. These source locations explain why the new leaf cannot simply
depend on today's evaluator or checker crates.

RT AD-004 (`quire-contract-runtime` commit `d610131`, document
`spec/assurance/AD-004-runtime-crate-layout.md`) records the earlier ported
`exact::CheckedPackage::{call,evaluate}` surface and its old Q-1 question.
Its fixed `MAX_CALL_DEPTH` / `DepthAboveMaximum` design is superseded by the
later QSL-682 / FR-262 contract and the no-depth-cap ruling in ADR-030. RT's
port is removed only when RT can consume the actual no_std implementation and
pass its own target acceptance; an interface plus std adapter alone is not
that migration.

### Interface flow

The core admits only checked evaluation input: either a checked expression
with completed parameters or a checked callable in its admitted closure with
completed arguments. The input remains tied to its checking authority,
callable identity, source locations and admitted object environment. A caller
cannot construct checked authority by registering a host function.

The interface exposes four semantic operations, whose exact Rust names and
representations are implementation details governed by FR-262:

1. Register a resumable host-body factory against one exact callable identity
   and the signature already admitted in the selected checked closure.
2. Begin one application with the checked input, admitted environment, one
   existing QSL meter, cancellation handle, and a registry bound to that exact
   closure.
3. Advance an opaque session by one finite saved body/task transition.
4. Resume a suspended body with either its initial arguments or the matching
   child's evaluation, and return a continuation, child request, or terminal
   evaluation to the scheduler.

The registry is open only for registration and is permanently sealed by the
first successful Begin. The core checks registration in the FR-262 order:
sealed registry, admitted identity, exact signature, duplicate identity, then
acceptance. A failed Begin does not seal it; successful Begin seals before
factory invocation or application charge. Registrations never invoke a
factory, consume application fuel, replace an existing binding, or alter the
checked closure. Each accepted factory creates fresh per-call state. The same
sealed registry is reused across sessions; selecting different bodies requires
a fresh registry.

The session owns its explicit task stack, call frames, saved lexical values,
package/import context, call-site locations, losses and cumulative
accounting. A generated host body returns a child request and saved continuation
to the scheduler instead of recursively calling the evaluator. The scheduler
admits the child, charges it, pushes its frame, then delivers the result to the
matching continuation exactly once. Every yield commits state. A scheduling
boundary is not an outcome or a charge point. Synchronous QSL `call` and
`evaluate` drive this same session to terminal. RT and CG call the shared
interface directly; neither receives a wrapper at RT's old `exact::` path nor
depends on evaluator internals.

### Depth, fuel and outcomes

The evaluator uses an explicit heap task/frame stack and does not recurse on
the native stack in proportion to expression or call depth. There is no
`MAX_*DEPTH`, call-depth setting, compiler ceiling, or depth refusal. Work
fuel is the existing `work_units` counter in `quire.value.accounting/v1`, not a
second scheduler budget. Each function application charges the existing
`function.call` point exactly once. Source arguments are evaluated
left-to-right before completed-input validation; validation precedes the call
charge, which precedes parameter binding and body execution. A checked
expression root with no application incurs no call charge.

Yielding, saving a continuation, pushing or popping a frame, and delivering a
child result add no charge. Existing kernel charge points and order remain
unchanged. A denied charge produces the existing located `incomplete` result
at that point, retains earlier spend, and does not record the denied charge.
Cancellation is observed at each charge and before each scheduler transition.
Terminal outcomes drop suspended continuations without executing them. These
rules apply identically to the reference and no_std implementations.

Deep acceptance is not a fixed-cap smoke test. FR-262 requires a recursive
`count` call at depth 100,000 to complete on a 512 KiB stack when its measured
fuel fits, to stop as `incomplete { limit_kind: work_units }` one unit below
measured spend, and to complete when raised to that spend. The same no-depth
rule covers nested values and `ValueType`, and no host stack overflow is an
accepted runtime outcome. The std and no_std implementations must agree on
outcome, refusal, location/loss behavior, and charge sequence for shared
inputs.

### Staged ownership and blockers

Linear was re-read on 2026-10-22. The following dependency states are the
structured blocker graph, not just the prose saying that no_std work is
parallel to MVP:

| Slice | Current state / owner | Blocked by | Earliest start and scope |
| --- | --- | --- | --- |
| IR-583 interface | In Progress / peter | STD-298, QSL-682 (Done) | QSL owns the interface, leaf-crate contract, reference adapter plan, and this design. |
| IR-585 S1 vocabulary | Backlog / unassigned | STD-298, IR-583, IR-269 | After these blockers: move evaluation refusal/call-plan vocabulary into `quire-semantic-value`, rebase object environment on `ObjectClosure`, remove the obsolete consumer fields. |
| IR-586 S2 checked terms | Backlog / unassigned | STD-297 | After its spec blocker: separate checked IR from checker; preserve checking authority, source locations and identities. |
| IR-587 S3 foundation | Backlog / unassigned | STD-299 | After its spec blocker: make `qsl-foundation` no_std + alloc capable while keeping `std` on by default. Build contract, target and gate: NFR-015 (closure tier, `thumbv7em-none-eabi`, `make build-no-std`). The foundation items the leaf itself uses move with S1, not S3. |
| IR-588 S4 semantics | Backlog / unassigned | STD-299, IR-586 | After both blockers: convert only evaluator-needed semantics; replace nondeterministic `HashMap` traversal in `model/population.rs` with `BTreeMap`. |
| IR-589 S5 package | Backlog / unassigned | STD-299 | After its spec blocker: make `qsl-package` no_std-capable and remove `serde_json` from checked-IR path; verify uncertain dependencies before choosing replacements. NFR-015 defines the checked-IR path (the in-memory checked closure, not the v2 wire), puts `quire-contract-model`, the v2 emitter, the I2 reader and the dependency-bearing constructor behind `std`, and sets the crate tier; the closure tier also needs S4 and `no_std` layers 2 and 1. |
| IR-590 S6 evaluator | Backlog / unassigned | STD-299, IR-583, IR-585, IR-586, IR-587, IR-588, IR-589 | After the interface, S1-S5, and its spec blocker: convert `qsl-eval` to no_std + alloc behind the interface; keep std enabled by default until S9. IR-269 is reached through S1, not a direct S6 edge. |

The structured graph leaves S2, S3 and S5 independent of the IR-269 MVP gate
and the IR-583 interface; each waits for its named spec blocker. S4 waits for
S2 and STD-299. S1 still waits for IR-269, IR-583 and STD-298. The suggested
owner lane is the maintainer of each owning QSL crate, coordinated by the QSL
architecture lead. S6 waits for S1-S5, IR-583 and STD-299; it reaches IR-269
through S1. The prose "parallel project" does not change those structured
dependencies.

After S6, S7 runs differential tests over identical inputs through both
implementations, S8 performs the unsafe/dependency audit (all newly authored
crates still use `unsafe_code = forbid`), and S9 switches the default and
retire/demotes the std path. S7-S9 remain separately gated slices, not part of
S6's claimed completion.

### RT and CG acceptance

IR-583 can serve std consumers once the interface and QSL reference adapter
exist. Complete RT migration requires the IR-590 implementation, not merely
the interface. RT acceptance therefore requires all of the following:

- Build the shared evaluator for RT's governed `thumbv7em-none-eabi`
  `no_std + alloc` target with its `std` feature disabled and without a
  QSL-local evaluator dependency.
- Exercise checked-expression evaluation and generated host-body execution
  through the shared session, including child handoff, preserved locals,
  cancellation, work-fuel denial, source location and loss propagation.
- Run the 100,000-deep recursive call at fitting `work_units` on a 512 KiB
  host stack, plus the `w - 1` denial and `w` restoration controls; verify
  iterative value/type handling at the depth required by FR-262.
- Differentially compare the shared implementation with the QSL std
  reference on the common evaluation corpus, including charge order and
  complete outcome data.
- Remove RT's evaluator port and do not add a compatibility re-export or
  shim at its former `exact::CheckedPackage` path. CG's generated closures
  remain a first-class lowering and consume the same shared interface rather
  than QSL or RT evaluator internals.

The target build, RT gate, memory/footprint limits and toolchain details are
owned by RT's consumer acceptance contract and must be recorded there; this
design does not claim those checks have run. Interface acceptance is not
evidence that the RT target accepts the later implementation.

## Decisions

- The QSL-owned leaf is the one shared interface and scheduler authority for
  interpreted checked expressions and registered generated bodies.
- QSL's std evaluator is the semantic reference until the no_std implementation
  passes the shared differential suite; only then may IR-593 switch defaults.
- Checked-term separation (IR-586) can proceed after STD-297 independently of
  IR-583. The no_std implementation (IR-590) still waits for S1-S5 and the
  shared interface (IR-583).
- The evaluator has no fixed depth cap. Resource limits and `work_units` fuel
  are the only bounds; scheduler steps are not a second fuel or externally
  visible outcome.
- RT and CG depend on the shared interface directly. A forwarding shim at the
  RT `exact::` path is prohibited.

## Risks

- The actual crate name, repository, public Rust item names and dependency
  versions are implementation choices not fixed by FR-262. The QSL owner must
  make them before the IR-583 implementation starts; license, CLA and content
  rights checks must precede publication of a new public repository.
- Today's checked graph is still owned by the checker and contains QSL-specific
  source/form dependencies. The interface must not solve this by importing the
  checker into the leaf. IR-586 must provide the stable checked-term boundary
  used by S6; the IR-583 public contract should keep the representation opaque
  until then.
- RT AD-004 predates FR-262 and describes a fixed call-depth guard and a
  different Q-1 fork. RT must amend that document and its FR-273 acceptance
  before claiming that the new interface completes the old port migration.
- IR-269 directly blocks S1 and indirectly precedes S6 through S1. It does not
  block S2-S5 directly. The structured Linear edges, rather than this design's
  earlier 2026-10-10 table, decide readiness.
- The `no_std` dependency verdicts, measured on QSL `origin/main`: `serde`,
  `sha2` and `thiserror` build without `std`. `quire-contract-model` does
  not (it depends on `serde_json` with `std`, `jsonschema`,
  `serde_path_to_error`, `serde_stacker` and `stacker`), so it stays behind
  `qsl-package`'s `std` feature, and with `std` off the checked package reads
  no `quire.checked-package/v2` JSON and recomputes no dependency
  `package_id`. `logos` and `serde_json` are on the evaluator closure:
  `qsl-semantics` has a normal edge to `qsl-forms` (layer 3 depends on
  layer 2), `qsl-forms` to `qsl-cst`, and `qsl-cst` needs `logos` and
  `serde_json`; `qsl-semantics` also depends on `serde_json`, `tempfile`
  and the FCD crates (`agent-ix-extraction-frontend`,
  `agent-ix-semantic-ir`). The closure tier of `qsl-package` therefore needs
  `qsl-semantics`, `qsl-forms` and `qsl-cst` each `no_std`-capable with
  those dependencies `no_std` or behind their `std` features; the layer 3 →
  2 edge stays. A `no_std` constructor of a dependency-bearing checked
  package needs an IR-owned `no_std` preimage reader built on
  `quire-canonical`'s `read`; NFR-015 does not provide one.

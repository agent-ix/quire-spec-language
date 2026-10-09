---
id: ADR-029
title: "Lifecycle, CLI, provider, plugin, cache and code-generation boundaries (ARCH-50)"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-073
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-076
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-300
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-305
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-306
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-339
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: relates_to
---
# ADR-029: Lifecycle, CLI, provider, plugin, cache and code-generation boundaries (ARCH-50)

## Status

Proposed, 2026-10-01. FR-275 to FR-299, with use cases US-029 and US-030,
specify this design, and FR-027 and FR-100 are amended in place (FR-027-AC-11,
FR-100-AC-10, FR-100-AC-11). The owning tickets are listed under References.
It decides the lifecycle and CLI orchestration
design that ADR-011 §5 leaves open (the command outcome fields, the outcome
kinds and their exit codes, and the shape of the orchestrating driver), and
the plugin, cache, execution-backend, inspection and rendering boundaries.
It also names the first bounded slice and the typed library operations
(§10, §3). The owner's rulings on the
draft's questions are in §11.

Item ids `LC-`, `CB-`, `OP-`, `PV-`, `PL-`, `CA-`, `EB-`, `IN-` and `SL-` are
local to this record. Other artifacts cite them as `ADR-029 PL-3`. "QSpec
FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement.

## Context

Measured on QSL `main` and quire-driver `main` on 2026-10-01.

**What exists.**

- The QSL root crate builds the binary `quire-spec` with five verbs:
  `parse`, `format`, `run`, `compile` and `lower` (`src/cli.rs`,
  `src/main.rs`). `run` and `compile` read native-run/1 and native-compile/1
  request files and route by edition (FR-027, FR-100). `lower` and the
  native paths are SEAM-1 and retire with ADR-011 M-6c.
- Exit codes follow QSpec FR-301's six values, but the mapping is spread
  across `Code::exit_code`, `RunError::exit_code`, `ClauseRunReport::exit_code`
  and literal codes in `main.rs` (`20`, `21`, `30`), so no single function
  owns it.
- The stage entries live in `qsl_replay::spine` (`compile`, `run`,
  `run_clause`) and `qsl_replay::replay`. Each takes explicit limits. None
  takes a cancellation input.
- The registry is QSL layer R, `qsl_route::Registry` (FR-075, FR-076,
  FR-080). The driver builds it from compiled-in descriptors and passes it
  as a value (ADR-012 §7.1).
- quire-driver is a library crate only (`src/drive.rs`, `registry.rs`,
  `handoff.rs`): S1 to S4, E4 emit, the `route` candidate step and CG
  negotiation. It has no binary.
- There is no plugin host, no cache, no inspection view other than `format`
  and the JSON outcomes, and no JIT. CG generates Rust from S5 IR for Kani
  harnesses, which is ahead-of-time generation in practice.

**What QSpec already requires.** QSpec FR-300 lists the typed lifecycle
operations and requires limits and cancellation on every request. QSpec
FR-301 fixes the exit codes and the severity order for multi-item output.
QSpec FR-305 allows two plugin forms, compile-time Rust traits and a
separate process speaking a canonical wire, and reads plugin output as
untrusted input. QSpec FR-306 requires per-node negotiation before emission,
cache keys over every meaning input, verification on read, and parity
between compiled and native execution. QSpec FR-339-AC-3 requires that
installing a backend never changes syntax.

**Owner rulings, 2026-10-01**, recorded on the owning ticket and applied
here:

1. The user CLI is the orchestrating driver (ADR-011 T-13). The qualified
   core, check and prove, stays separable into a narrowly scoped tool,
   because a tool-qualification audit may require two binaries.
2. `analyze` and offline `monitor` (checking a supplied trace) are QSL
   library operations. Generating a runtime monitor goes through CG and RT
   via the driver. The driver CLI exposes all three.
3. One pluggable execution-backend seam holds the interpreter, AOT and JIT,
   with result parity and checked-only input. This record specifies the seam
   and the JIT semantics. The JIT is built after the engines stabilize, and
   a benchmark picks its code generator.
4. First-party backends are compile-time Rust traits. Third-party providers
   use one out-of-process wire. A plugin's `proved` result cannot be
   replayed, so it is labelled trusted.

## Decision

### 1. Lifecycle

**LC-1 Stages and owners.** A user goes through these stages. Each has one
owner. "Library" means a QSL library crate; "driver" means the orchestrating
driver library, which sits downstream of CG; "provider" means a backend
reached through the registry.

| Stage | What the user does | Owner | Library operation (§3) | CLI verb |
|---|---|---|---|---|
| Author | Write and format source; see syntax errors as they type | QSL library: S1 `cst` and layer-1 `format`, tool layer `complete::editor` | `parse`, `format` | `parse`, `format` |
| Check | Admit models and libraries, check the unit, link the checked closure | QSL library: I1, I2, S2 to S4 | `select`, `check`, `check_fences` | `check` |
| Compile | Produce the in-process `CheckedPackage` | QSL library: S1 to S4 (`qsl_replay::spine`) | `check` | `check` |
| Package | Emit `quire.checked-package/v2` bytes with the digest-addressed source provision another party needs to import (I2) or replay (E9) | QSL library: S4 `package` (E4) | `package` | `compile` |
| Execute | Call a checked function or evaluate a checked clause | QSL library: layer 5, through the execution-backend seam (§7) | `execute` | `run` |
| Analyze | Decide a claim over every behaviour of a model with an in-process engine | QSL library: the engines in layer A (CB-3), outside the qualified core; their certificate checkers in layer 6, inside it (CB-2) | `analyze` | `analyze` |
| Monitor | Check a supplied trace against temporal clauses | QSL library: layer A over the layer-5 trace evaluator (ADR-014 A-4) | `monitor` | `monitor` |
| Prove | Negotiate, generate, run a backend, replay counterexamples, settle terminal records | Driver, with CG, IR and RT, and providers | `lower`, `generate`, `prove` | `prove` |
| Replay | Re-execute a counterexample through S6a | QSL library: layer 6 `replay` | `replay` | `replay` |
| Generate | Emit an AOT crate or a runtime monitor from S5 IR | Driver, with CG and RT | `lower`, `generate` | `generate` |
| Inspect | See a package, a node, an outcome, a counterexample, a trace or a state graph | QSL library: layer P (§9) | `inspect`, `render` | `inspect` |

**LC-2 One request and outcome shape.** Every library operation takes a
typed request, explicit limits and a cancellation handle, and returns
`Result<Staged<T>, StageFailure<C>>` (ADR-013 T-4), with `T` the operation's
output and `C` its cause type. Two operations keep their existing shapes:
`execute` returns `Result<Evaluation<Value>, CallFailure>` (ADR-011 §2.3), and
`prove` and `analyze` return one FR-331 terminal record per requested item
(ADR-013 O-24) inside `Staged`. Every operation is a pure function of its
request except for cancellation.

**LC-3 Cancellation.** `Cancel` is a cloneable handle the caller owns. An
operation polls it wherever it charges a work meter or a stage limit, so a
cancelled run stops within one charge. A cancelled operation returns the
typed failure `StageFailure::Cancelled(CancelCause)`, where `CancelCause` is
`Requested` or `Deadline`. `Deadline` is set by the frontend that owns the
timer; the library reads no clock. A cancelled operation emits no partial
artifact. Its O-16 category is incomplete, so it exits 22. A cancelled
`execute` returns `CallFailure::Cancelled`. A cancelled item inside `prove`
or `analyze` settles FR-331 `incomplete` with its cause kept (ADR-013 O-16).
This adds a fourth `StageFailure` variant and a third `CallFailure` variant
(Amendments, below).

**LC-4 Limits.** Every limit is caller-configurable. Each has a default the
caller can replace, and none is fixed. A limit that is reached refuses with
`LimitExceeded` naming the limit (ADR-013 T-4), and is never read as success.
No stage recurses on the native stack, and depth is not a limit: node,
byte and work limits bound every stage (ADR-030 RU-1). The fixed ceiling
`quire_semantic_value::checking::MAX_CHECKING_DEPTH` and its
`DepthAboveMaximum` refusal are deleted (Amendments, below).

### 2. CLI and the qualified core

**CB-1 The CLI is the driver's.** The user binary is `quire`, built from the
driver repository. It is a thin frontend: it parses arguments, reads the
files the arguments name, builds a typed request, calls one library
operation, renders the outcome and maps it to an exit code. It makes no
semantic decision and reads no diagnostic text (ADR-011 FB-09). The QSL root
crate's `cli` and `main`, and the `quire-spec` binary, are deleted in the
change that lands the driver verbs that replace them (`parse`, `format`,
`run`, `compile`), as ADR-011's no-gap ruling requires. The request handling
in QSL `command` that FR-027 and FR-100 specify stays a library operation,
reached by the driver.

**CB-2 The qualified core.** The qualified core is the check path (S0 to S4),
the prove path (E5 to S8, with replay through S6a), and the certificate
checkers through which `analyze` enters the core (RU-2). The checkers are
the zone certificate checker (ADR-026 CF-*), the EN-5 certificate checker
(ADR-028 CE-*) and the closure checks, such as ADR-022's trap
re-exploration. Each is a layer-6 entry beside `replay`. It recompiles the
package and checks the certificate with its own code, reading nothing from
the engine's search but the certificate. The engines that produce
certificates are outside the core. An `analyze` `proved` counts only once
its in-core checker accepts the certificate. The core's crates are:

- in QSL: `qsl-foundation`,
  `qsl-cst`, `qsl-source`, `qsl-forms`, `qsl-semantics`, `qsl-package`,
  `qsl-eval`, `qsl-route` and `qsl-replay`, which holds the certificate
  checkers (the `maybe_grow` wrapper `qsl-walk-grow` is outside the core);
- the exact-value kernel `quire-exact` and the semantic-value leaf
  `quire-semantic-value`, each in its own repository (`agent-ix/quire-exact`,
  `agent-ix/quire-semantic-value`), which the core's QSL crates depend on;
- the walker toolkit `quire-walk` (ADR-030 FR-356), in its own repository,
  `agent-ix/quire-walk`, which the core's `qsl-forms` and `qsl-semantics`
  depend on;
- in the driver repository: the driver library `quire-driver`;
- the CG, IR and RT crates that the prove path reaches, under their own
  records.

**CB-3 What "separable" means.** Separability is a property of crates and
their dependency closure:

1. **No frontend in the core.** No core crate depends on an argument parser,
   a terminal or colour crate, a renderer other than the JSON serialization
   of its own outcome types, the plugin host, the cache, an execution backend
   other than the S6a interpreter, or any crate outside CB-2. A direction
   check over `cargo tree` of each core crate enforces this, with the same
   tooling as ADR-011 T-12.
2. **No ambient input in the core.** A core crate reads no environment
   variable, configuration file, search path, clock or global registry. It
   writes nothing to stdout or stderr, never calls `std::process::exit`, and
   returns a typed outcome with no panic across its public boundary. The
   bytes it reads arrive in its request, by digest where a digest names them
   (ADR-013 O-26). The prove path runs a backend tool only at the path the
   request names (ADR-012 §7.4).
3. **Non-core crates sit above the core.** The crates this record adds sit
   outside CB-2 and above it:
   - QSL layer A, crate `qsl-analyze`: the `analyze` and `monitor`
     operations and the in-process engines (model checking, statistical,
     EN-5 and zone search). It depends on layers 6, 5, 4, 3, F, SV and K.
     The engines the temporal and probabilistic drafts place in layer 5
     move here (Amendments, below), so `qsl-eval` keeps only S6a, the
     trace evaluator, simulation and the execution seam.
   - QSL layer P, crate `qsl-inspect`: inspection views and the text and DOT
     renderers (§9). It depends on layers 6, 5, 4, 3, F, SV and K.
   - QSL layer X, crate `qsl-jit`: the JIT execution backend (§7). It
     depends on layers 5, 4, 3, F, SV, K and the chosen code generator.
   - Driver repository: `quire-plugin-host` (§5), `quire-cache` (§6),
     `quire-aot` (§7) and the binary crate `quire-cli`.

   Nothing in CB-2 depends on any of them.
4. **A second binary costs no core change.** A qualified binary is a thin
   frontend over the core with the verbs `check`, `compile`, `prove`,
   `replay`, `check-certificate` and `version`, JSON output and the CB-4
   exit function. `check-certificate` runs a certificate checker over a
   certificate an `analyze` run wrote, so a certificate produced outside the
   qualified binary is accepted or rejected inside it. It links no crate
   from item 3. If an audit requires it, it is added as a second
   binary crate in the driver repository and no core crate changes.

**CB-4 Outcome document and exit codes.** Every verb writes one JSON outcome
document to stdout in machine mode. Its members are:

- `format`: the outcome document's format identity, for example
  `quire-outcome/1`. It names the shape of the document.
- `operation`: the library operation the verb called.
- `last_stage`: the last ADR-011 stage the operation reached.
- `category`: the ADR-013 O-16 category of the whole outcome.
- `items`: for `prove`, `analyze` and `monitor`, one entry per requested item
  with its terminal record and category. A proof item (`prove`, `analyze`)
  never carries the `undefined` label: an undefined claim evaluation settles
  it `refuted` with cause `UndefinedEvaluation{where, cause}`, category
  violation. A `monitor` clause whose evaluation over the supplied trace is
  undefined is a violation with that cause.
- `diagnostics`: each with its typed cause, catalog code, `Locus` (ADR-013
  T-5) and message.
- `artifacts`: the identities of the artifacts produced, such as a
  `package_id` or a generated artifact's content identity.
- `result`: an `execute` outcome's completed value, undefined reason or
  exhausted limit (FR-286); `null` for every other outcome.

The JSON document is the serialized library outcome, so the same request
through the library and through the CLI gives the same document (QSpec
FR-300-AC-3).

One total function maps an O-16 category to an exit code. It has no `_` arm
and denies `clippy::wildcard_enum_match_arm` (ADR-011 §5):

| O-16 category | Exit code |
|---|---|
| success | 0 |
| violation | 10 |
| undefined (RU-1; a non-proof evaluation item keeps the label `undefined`) | 10 |
| refusal | 20 |
| unsupported | 21 |
| incomplete (limit, cancellation, deadline, budget) | 22 |
| inconclusive: a proof or `analyze` item (incomplete under QSpec FR-301) | 22 |
| inconclusive: a supplied-trace clause pending when the trace ends (completed without violation) | 0 |
| internal failure (including tool failure) | 30 |

A multi-item outcome exits with the most severe code present, in QSpec
FR-301's order: 30, 20, 21, 22, 10, 0. `StageFailure::Refused` takes the O-16
category of its cause, so a profile-gated construct is unsupported (21) and
any other refusal is refusal (20). `StageFailure::Limit` and
`StageFailure::Cancelled` are incomplete (22). `StageFailure::Fault` is
internal failure (30). A write failure on stdout or stderr is internal
failure (30), and a broken pipe ends quietly with the outcome's own code.
Every existing local exit mapping is deleted when this function lands.

**CB-5 Verbs and arguments.** The verbs are `parse`, `format`, `check`,
`compile`, `run`, `analyze`, `monitor`, `prove`, `replay`, `generate`,
`inspect` and `version`. `check` calls `check_fences` when its arguments
name a spec artifact inventory and the module manifests that type it, and
`select` then `check` when they name a unit source. Every verb takes
`--format json|text`, and
`inspect` also takes `dot` for graph views. An unknown verb, option, profile
or provider refuses before any stage runs and names the argument (QSpec
FR-301-AC-3, exit 20). `version` prints the application version. The CLI
reads every input from the paths its arguments name, or from stdin, and
reads no configuration file the arguments do not name.

### 3. Library operations

**OP-1 The typed operations.** FR-027 and FR-100 are the base: their request
documents are the CLI encodings of `package` and `execute`. Each operation
below takes its request, its limits and `&Cancel`.

| Operation | Role | Owner | Input | Output |
|---|---|---|---|---|
| `parse` | Read source into syntax | QSL S1, S2 | source bytes, `SourceIdentity`, S1 and S2 limits | `Staged<ParsedSource>`; tooling reads S1's `LosslessCst` directly |
| `format` | Format admissible source (FR-003) | QSL layer 1, `qsl_cst::format` | an admissible `ParsedSource`, format limits | `Staged<FormattedSource>`: the formatted bytes |
| `select` | Admit the domain packages a unit's `model` declarations select | QSL I1 `model::intake` | domain package documents by digest, the unit's model selections | `Staged<AdmittedModels>` |
| `check` | Check and link | QSL S3, S4 | `ParsedSource`, `AdmittedModels`, the dependency input (ADR-015 D-1), lock evidence (ADR-011 §2.4) | `Staged<CheckedPackage>` |
| `check_fences` | Check every `quire` fence of a spec artifact inventory as a state invariant of the object its artifact declares (FR-355) | QSL I1 `model::intake`, S1 to S3 | the artifact inventory, the module manifests that type it | `Staged<FenceReport>` |
| `package` | Emit the checked package | QSL S4 `package` (E4) | `&CheckedPackage` | `Staged<EmittedPackage>` with its source provision |
| `execute` | Call a checked function or evaluate a checked clause | QSL layer 5, through §7's seam | `&CheckedPackage`, a `QualifiedName` or clause selection, arguments, `ObjectEnvironment`, accounting limits, the chosen execution backend | `Result<Evaluation<Value>, CallFailure>` |
| `monitor` | Check a supplied trace offline | QSL layer A | `&CheckedPackage`, clause selections, a trace (§3 OP-3) | `Staged<MonitorOutcome>` |
| `analyze` | Decide claims over every behaviour with an in-process engine | QSL layer A | `&CheckedPackage`, claim items, subject binding, engine limits | `Staged<AnalyzeOutcome>` |
| `lower` | Read the emitted package into target-neutral IR | Driver, with the IR reader (E5) | `EmittedPackage` and its expected `package_id` | IR nodes, or IR's typed refusal |
| `generate` | Emit backend artifacts from IR | Driver, with CG and RT | IR nodes, the routed `BackendId` per item, the generation target (proof harness, AOT crate, runtime monitor) | FR-331 `artifacts` with source maps; an unsupported item emits nothing |
| `prove` | Negotiate, generate, run, replay and settle | Driver | `EmittedPackage` from the same run's `check`, claim items, the registry value, provider limits | `Staged<ProveOutcome>`: one FR-331 terminal record per item |
| `replay` | Re-execute a counterexample | QSL layer 6 | `ReplayRequest` with its byte provision (ADR-013 O-26) | `Staged<ReplayResult>` (ADR-013 O-27) |
| `inspect` | Build a typed view | QSL layer P | a package, outcome or graph, and a view selector | `Staged<View>` (§9) |
| `render` | Turn an outcome or view into text or DOT | QSL layer P | an outcome or `View`, the source byte provision | bytes |

Each request names its source and artifact identity, its profile and its
limits, and each later operation requires its exact predecessor type, so a
wrong-stage input fails to compile (QSpec FR-300-AC-2).

**OP-2 `analyze`.** It runs QSL's in-process engines only: ADR-018's EN-1,
ADR-024's EN-4, ADR-028's EN-5, ADR-026's zone search, and later QSL engines.
SMT engines are IR's backend and run through the driver's `prove` (RU-3).

- **Input.** The checked package; the claim items, each named by its
  occurrence key; the subject binding (the `ModelSystem` and the initial
  population, FR-120); each engine's request settings and budgets (for
  example ADR-018's model-check limits); the accounting limits; `&Cancel`.
- **Output.** `AnalyzeOutcome`: one terminal record per requested item, with
  its O-16 category, its typed cause, its basis, its counterexample or
  certificate where it has one, and its accounting.
- **Terminal records.** Exactly one per requested item (AD-016's
  terminal-disposition rule), with the FR-331 result vocabulary. An item no
  QSL engine accepts settles `unsupported` with a typed cause. A `proved`
  settles only after the layer-6 certificate checker for its engine accepts
  the certificate; a rejected certificate settles `inconclusive` with cause
  `CertificateRejected` (ADR-026 CF-4). A proof from an engine with no
  certificate checker in the core settles `proved`, category success,
  labelled `uncertified` and naming the engine (RU-6). A
  counterexample settles `refuted` only after `analyze` replays it through
  layer-6 `replay` and the replay reproduces it (ADR-011 FB-07). Otherwise
  it settles `inconclusive` with cause `replay_parity` or `replay_refused`,
  as ADR-013 O-16 states. An evaluation of the claim that is undefined at a
  state or step, in any engine, settles the item `refuted` with cause
  `UndefinedEvaluation{where, cause}`, once replay reproduces the undefined
  value there; the item is category violation, exits 10 and carries no
  `undefined` label. Budget exhaustion and cancellation settle
  `incomplete` with the cause kept.
- **Who settles.** QSL settles the terminal records of its native engines,
  and of third-party plugins (PL-5), in layer 6 `qsl-replay`: the
  certificate check, the replay of a counterexample and the certification. CG
  keeps the C-09 map that settles a Kani outcome with its E9 replay result
  (ADR-013 C-09), and the orchestrating driver runs it (ADR-011 T-13).
- **One certification per proof.** Every `proved` carries exactly one
  certification (ADR-018 PC-1). A native-engine proof is `certified` once
  its core checker accepts, or `uncertified` when no core checker exists
  (RU-6). A plugin proof is `trusted` (RU-4). A Kani `proved` is
  `certified`: the Kani prove path is part of the qualified core (CB-2).
- **Inside `prove`.** QSL publishes a provider manifest for each engine. When
  negotiation routes an item to one, the driver calls `analyze` in process as
  that provider, so the two paths produce the same record.

**OP-3 Offline `monitor`.** It applies the layer-5 trace evaluator to a
trace the caller supplies. It is the same semantics replay uses (ADR-014
A-4).

- **Input.** The checked package; one or more temporal clause selections; a
  trace document holding a finite trace or a lasso (prefix positions, then a
  non-empty loop), whose positions are admitted as snapshots and invocations
  (FR-106) before any evaluation; the evaluator limits; `&Cancel`.
- **Output.** `MonitorOutcome`: one record per clause. The record holds the
  verdict under ADR-014 A-4. Over a finite trace the verdict is a violation
  at a position, with the QSpec FR-351 separating witness, or pending. Over a
  lasso it is a violation or `tested`.
- **Terminal records.** A violation is category violation (exit 10). An
  evaluation undefined at a position is a violation there, with cause
  `UndefinedEvaluation{where, cause}` (exit 10). Pending at the trace's end
  is inconclusive and exits 0: no violation was observed. `tested` is success (0) and is evidence for that trace only, never
  `proved`. A supplied trace carries no enabledness, so a clause with a
  non-empty fairness set settles unsupported, cause
  `unsupported_projection`/`missing-fairness-premise` (21), as QSpec FR-362
  states. A trace that fails admission refuses before any clause is
  evaluated.
- **Generated monitors.** A runtime monitor is a `generate` target, owned by
  the driver with CG and RT. It reads the same S5 IR and is held to the same
  verdicts as `monitor` on a shared trace corpus.

### 4. Providers

**PV-1 Registration.** The registry is the layer-R `qsl_route::Registry`
value, and nothing else registers backends (FR-075, FR-080). The driver
builds it from two sources: the compile-time providers linked into the
binary, each giving its FR-331 manifest through the `Provider` trait (PV-3),
and the plugin processes the caller names (§5), each giving its manifest in
its `hello` frame. Every manifest becomes a `BackendDescriptor` by the same
conversion. The registry reaches each consumer as an argument (ADR-012
§7.1).

The descriptor carries a typed origin, `ProviderOrigin::Linked` or
`ProviderOrigin::Process`, owned by layer R. The conversion sets it:
`Linked` for a compile-time provider, `Process` for a plugin `hello`. Two
descriptors of one `BackendId` that differ in origin differ in a member, so
they conflict under the registration rule: both are withdrawn, `duplicate-backend`
is recorded for each conflicting manifest digest, and the identity stays
unregistered for the run (FR-288). The origin is a driver-supplied fact held in
the registry. A manifest never states it, so a plugin cannot claim to be
linked. A byte-identical registration stays idempotent, and the outcome does
not depend on registration order (PV-2 item 2). The built-in `kani` and a
plugin manifest declaring `kani` therefore neither wins: the identity is
unregistered and its items have no candidate and decline.

**PV-2 Negotiation leaves results unchanged.** Installing, removing or
reordering providers changes results only through each item's candidate set,
and only for items whose capability kind the change touches:

1. **Front end.** S1 to S4 and `package` take no registry, so their output
   is independent of it by signature. Compiling one source with an empty
   registry and with a populated one gives byte-equal v2 bytes and the same
   check outcome (QSpec FR-339-AC-3).
2. **Order.** Two registries built from the same manifests in any order are
   equal and give identical candidate sets (FR-080).
3. **Locality.** An item's terminal record is a function of the item, its
   candidate set and the routed provider's run. Adding a provider that does
   not advertise an item's kind leaves that item's record unchanged.
4. **Meaning.** A `refuted` record always comes from a counterexample that S6a
   replay reproduced, whichever provider produced it. A `proved` record names
   the provider that produced it.

**PV-3 The provider trait.** First-party providers implement one trait in
the driver library:

```rust
pub trait Provider {
    /// The FR-331 provider manifest this provider advertises.
    fn manifest(&self) -> ProviderManifest;
    /// Run the routed items of one FR-331 request.
    fn run(
        &self,
        package: &EmittedPackage,
        request: &ProviderRequest,
        limits: &ProviderLimits,
        cancel: &Cancel,
    ) -> ProviderResults;
}
```

The Kani backend reached through CG, the QSL engines of OP-2 and the plugin
host's process adapter (§5) each implement it, so the driver handles every
provider the same way after routing.

**PV-4 Negotiation for a process provider.** ADR-012 §7.2 makes every
settlement an arm of CG `negotiate_*` over a closed backend kind. A
third-party provider cannot add an arm, so CG's closed kind gains one
variant for process providers, `BackendKind::Process(BackendId)`. Rulings:

1. **Manifest checked in full.** Its arm settles from the provider's manifest
   alone, against the item's extent classification. QSpec FR-290 owns the
   check order and every disposition and cause; this ADR does not restate
   them. The descriptor CG receives is a QSpec FR-331 backend-provider/v1
   `BackendDescriptor` (`proposals/backend-provider-v1/schema.json`). It
   carries the `advertises` (kind, mode) pairs and `domains`, the ADR-014
   boundable domain kinds for which the backend accepts a finite bound. Its
   `bounds` are the published default of each limit the backend applies, not
   admission maxima. Every `domains` defect in a provider's hello is a
   per-registration refusal under FR-290,
   `invalid_capability`/`invalid-domains`. That
   descriptor is CG's own descriptor for the Process variant, not
   `qsl_route::BackendDescriptor`, which holds the admitted (kind, mode)
   pairs and the admitted `domains`. CG reads those `domains` through
   `BackendDescriptor::domains()` for FR-290's domain-kind routing step, which
   is CG's. It never calls the plugin.
2. **Identity is data in the variant.** The plugin's `BackendId` sits inside
   `Process(BackendId)`; it is never a new kind. `from_identity` keeps mapping
   the built-in static identities to their own kinds. CG settles a descriptor
   to `Process(id)` when its `ProviderOrigin` is `Process`. A `Linked`
   descriptor with an identity `from_identity` does not know is
   `UnknownBackend`. CG never infers `Process` from an unknown identity or
   from executable text.
3. **Only negotiation is CG's.** Generation yields no CG artifact
   (`KindOutput::Process`, empty), because the plugin receives the v2 package
   bytes, not generated code (PL-4). CG has no adapter and no execution for a
   process provider: the driver's plugin host runs the PV-3 process adapter.
   CG has no terminal step: the driver reads the FR-331 result with the typed
   reader and settles per PL-7 (`refuted` only through S6a replay; `proved`
   carries the certification `trusted`). Each CG arm for `Process` is a typed
   pass-through or empty output, never a panic.
4. **Disposition from the manifest alone.** A `Process(id)` item settles
   under QSpec FR-290's "Single-candidate arm", whose step 2 is the
   domain-kind check (FR-290-AC-13), read from the descriptor in ruling 1
   and the item's extent. There is no default disposition.
5. **Origin, option (b).** `ProviderOrigin` on the descriptor (PV-1) is how CG
   learns that a descriptor is a process provider. Not (a), a driver-supplied
   map beside the descriptors: it is a second source of truth that can
   disagree with the registry. Not (c), a manifest member: origin is a fact
   the host knows, not something a provider says about itself, and a plugin
   must not be able to claim to be linked.

### 5. Plugins

**PL-1 Forms.** A plugin is either a compile-time Rust implementation of the
`Provider` trait, linked into the binary, or a third-party executable that
runs as its own operating-system process and speaks the plugin wire below.
The separate process is what the wire runs over. An installed plugin is a
trusted program, like any program the user installs, and runs with the
user's own authority. The host loads no dynamic library (QSpec FR-305).

**PL-2 Transport.** The host starts the plugin at the path the caller names.
It does no discovery and no search-path lookup. The plugin reads frames on
stdin and writes frames on stdout. Its stderr is captured as opaque text up
to the caller's byte limit and shown to people only; nothing parses it
(ADR-011 FB-02).

**PL-3 Framing.** A frame is an unsigned 64-bit big-endian length followed by
that many bytes of one RFC 8785 (JCS) JSON value, encoded with
`quire-canonical`. A reader re-encodes each value and refuses it if the
bytes differ, so a frame has exactly one encoding. The caller sets the
maximum frame length and the maximum total bytes per run. A frame above
either limit fails the plugin (PL-7).

**PL-4 Messages and protocol identity.** Every frame is one envelope:

```json
{"protocol": "quire.plugin-wire/v1", "id": 7, "kind": "request", "body": {}}
```

- `protocol` is the wire's identity. A reader accepts exactly one identity
  and refuses any other before reading the body (ADR-013 O-22). The wire
  carries no version range, executable digest or crate digest.
- `id` is a request id the host assigns. Every reply names the id it
  answers.
- `kind` is one of `hello`, `request`, `result`, `cancel`, `shutdown` and
  `fault`.
- `body` is typed per kind. A `request` body is an FR-331
  `quire.backend-provider/v1` request with the routed items, the v2 bytes
  of the package and the provider limits, including the deadline. A `result`
  body is an FR-331 result. A `hello` body is the plugin's FR-331 manifest.
  A `fault` body is a typed cause with a catalog code.

**PL-5 Capability advertisement.** The plugin's first frame is `hello`. Its
manifest lists the advertised (kind, mode) pairs, domains, options and
bounds in FR-331 form. The host takes the plugin's `BackendId` from the
manifest it received: the backend identity alone, verbatim (ADR-013 O-19).
The rest of the manifest gives the plugin's advertised capabilities (FR-288).

**PL-6 Lifecycle.** For one driver run, each named plugin goes through:
start, `hello`, registration (PV-1), zero or more `request` and `result`
exchanges for the items routed to it, `shutdown`, and exit. The host sends
`cancel` with the request id when the caller cancels. The plugin answers
with a `result` that settles each open item `incomplete` with the cause
cancelled. When the caller's grace period ends without that answer, the host
kills the process.

**PL-7 Run budgets and failure handling.**

- **Budgets.** The caller's run budgets apply to each plugin run: the
  deadline, the frame and total byte limits (PL-3), and the process memory
  and CPU-time limits where the platform provides them. They bound a run;
  they grant or withhold nothing.
- **Typed output.** Every `result` is read with the typed FR-331 reader.
  A plugin never produces a checked object: it receives v2 bytes, and IR's
  reader revalidates the `package_id` (ADR-011 §4).
- **Failures.** A crash, a non-zero exit, a malformed or non-canonical frame,
  a wrong protocol identity, an oversized frame, a result for an item that
  was not routed to it, a result for a kind it did not advertise, or a
  missing terminal record settles each affected item FR-331 `failed` with a
  typed tool-failure cause (exit 30). A deadline overrun kills the process
  and settles each open item `incomplete` with cause `TimedOut` (ADR-014
  B-5, exit 22). Items routed elsewhere are unaffected.
- **Counterexamples.** A plugin's counterexample is canonical assignments.
  The item settles `refuted` only when S6a replay reproduces it (PV-2
  item 4).
- **Trusted proofs.** A plugin's `proved` result cannot be replayed and has
  no in-core certificate checker. The
  terminal record keeps the result `proved`, names the plugin's `BackendId`,
  and carries the certification `trusted`. A renderer shows the label beside
  the verdict.

### 6. Cache

**CA-1 What is cached.** The cache holds results that are expensive and are a
function of their key: `prove` terminal records, `analyze` terminal records
and `generate` artifacts. Parsing, checking, packaging, execution and
monitoring are cheap and deterministic and run every time.

**CA-2 Key.** The key is one canonical content identity: a digest in a
QSpec FR-201 domain, `quire.cache-key/v1`, over the JCS bytes of:

- the operation (`prove`, `analyze` or `generate`);
- the subject's `package_id`. Its FR-322 preimage binds the source digests,
  profiles, model selections and each dependency's `package_id`;
- the canonical request: the items by occurrence key, the claims, the
  domains and bounds, the options that affect a result, and every
  deterministic budget that can move a result to `incomplete` (work units,
  state and transition budgets);
- the provider's `BackendId`, the identity string alone, and the provider
  manifest's content identity, the `ManifestDigest` that layer R computes over
  the FR-331 manifest's canonical bytes, in its domain, as the `manifest`
  member. A change to any manifest member, the tool version included,
  changes the key;
- for `generate`, the generation target and its options.

Paths, timestamps, wall-clock deadlines, application version strings and the
execution backend that ran are outside the key. Each either has no effect on
the result or is already bound by a member above.

**CA-3 Invalidation.** Any change to a meaning input changes the key, so a
changed input is a miss and a stale entry is never read. Nothing else
invalidates entries. There is no index, manifest or ledger: the store is the
set of entries, each named by its key.

**CA-4 Store and read.** An entry holds the key's JCS preimage and the
stored value: the terminal records' result-identity members, or the artifact
bytes with their source maps. A read accepts an entry only when the entry's
name, the digest of its stored preimage and the requested key are equal, and
the value parses with the same typed reader that reads a fresh result.
Anything else is a miss. A writer writes to a temporary name and renames it
into place. Two writers of one key write identical bytes, so a race is
harmless. The store is a directory the caller names. Eviction keeps the
store under the caller's size limit.

**CA-5 What is never stored.** A result whose category is internal failure,
a cancelled or deadline result, a time-limited `incomplete`, and any member
that records wall time. A plugin's results are not stored (RU-5).

### 7. Execution backends, AOT and JIT

**EB-1 The seam.** One trait in layer 5 (`qsl-eval`, module `execution`)
holds every execution backend:

```rust
pub trait ExecutionBackend {
    type Prepared: PreparedEntries;
    /// Prepare the selected entries of one checked package. An entry this
    /// backend cannot run settles `Unsupported` for that entry only.
    fn prepare(
        &self,
        package: &CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared, PrepareFailure>;
}

pub trait PreparedEntries {
    /// The same contract as `CheckedPackageEvaluation::call`.
    fn call(
        &self,
        entry: &QualifiedName,
        arguments: &Arguments,
        environment: &ObjectEnvironment,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation<Value>, CallFailure>;
    /// The same contract as `CheckedPackageEvaluation::evaluate`.
    fn evaluate(
        &self,
        clause: &ClauseSelection,
        input: &ClauseInput,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation<Value>, CallFailure>;
}
```

`PrepareFailure` holds per-entry `Unsupported` causes, `Limit`, `Cancelled`
and `Fault`. Three backends implement the seam:

| Backend | Crate | `prepare` | `call` and `evaluate` |
|---|---|---|---|
| Interpreter | `qsl-eval` | Accepts every entry S6a accepts | S6a, `CheckedPackage::call` and `evaluate` |
| AOT | `quire-aot` (driver repository) | `lower`, then `generate` with the AOT target, then a build with the toolchain the caller names | Runs the built executable as a child process over PL-3 framing |
| JIT | `qsl-jit` (QSL layer X) | Translates the checked entries to machine code in memory | Calls the generated code |

**EB-2 Checked-only input.** `prepare` takes `&CheckedPackage`, which only S4
constructs (ADR-011 §4), so every backend admits checked input only, by
type. `generate` takes the `EmittedPackage` that the same run's `check`
produced, and the IR reader checks it against that run's expected
`package_id` (ADR-011 E5). Bytes read from a file are never a `generate` or
`prepare` input. A `compile_fail` test for each entry type is the evidence,
as for ADR-011 FB-03 and FB-04.

**EB-3 The interpreter is the reference.** S6a defines what a checked
package means. Replay runs only through S6a (ADR-011 FB-07). AOT and JIT are
faster ways to get the same result.

**EB-4 JIT semantics.** A JIT run returns the same `Evaluation` as the
interpreter for the same package, entry, arguments, environment and
accounting limits. In detail:

1. **Determinism.** The result depends only on those inputs. Generated code
   reads no clock, random source, environment or thread state, and visits
   collections in the interpreter's canonical order.
2. **Exact arithmetic.** Every value operation has `quire-exact` semantics:
   integers and rationals are exact, and decimal and IEEE operations follow
   the selected profile exactly. Generated code uses a machine-width fast
   path only where a checked bound shows the exact result fits. Every other
   operation calls the kernel.
3. **Outcomes.** `Completed`, `Undefined`, `Refused` and `Incomplete` match
   the interpreter's, with the same typed cause, the same `Location` and the
   same loss records.
4. **Budgets.** For every accounting limit, the JIT run is `Incomplete` with
   the same charge point and limit as the interpreter's run. The JIT may
   group charges if this holds.
5. **Limits.** Node, byte and work limits bound a run, checked explicitly.
   Depth is not a limit, and a native stack overflow is never the failure. Compile work
   and code size are bounded by `PrepareLimits`, and reaching one settles a
   `Limit` failure of `prepare`.
6. **Failures.** A JIT internal error is `Fault` (exit 30) and never a
   verdict. An entry the JIT cannot translate settles `Unsupported` for that
   entry, and the caller chooses whether to run it on another backend.
7. **No effects.** Generated code performs no I/O and calls only the kernel
   and the semantic-value runtime.

AOT meets the same seven rules. The outcome carries no member naming the
backend that ran, because parity makes the backend invisible in the result.

**EB-5 Parity evidence.** Each non-interpreter backend runs the conformance
corpus and generated programs against the interpreter, and any difference
fails the gate (QSpec FR-306-AC-1).

**EB-6 The AOT boundary.** QSL's code-generation boundary is the v2 package
(E5). CG generates Rust from S5 IR, negotiated per item before emission. An
item that is not settled `supported` emits nothing (QSpec FR-331-AC-2). The
generated artifact is FR-331 `artifacts` bytes with a source map keyed by
occurrence key. QSL emits no target-specific code. A runtime monitor is
another `generate` target behind the same boundary.

**EB-7 Building the JIT.** This record fixes the seam and EB-4. `qsl-jit` is
built after the engines stabilize. A benchmark picks the code generator,
Cranelift or LLVM, and EB-4 binds either one.

### 8. Selecting a backend

**EB-8** The caller picks the execution backend in the request. The CLI flag
is `--engine interpreter|aot|jit`, with `interpreter` as the default. The
three names are the seam's closed set. A name outside the set refuses as an
unknown argument (exit 20). A name in the set whose backend this binary does
not link refuses before any stage as unsupported (exit 21). The binary never
substitutes another backend.

### 9. Inspection and rendering

**IN-1 Views.** `inspect` returns typed views. Each view locates source
through the occurrence-keyed source map and never by reparsing text (ADR-011
FB-01).

| View | Shows |
|---|---|
| Package | `package_id`, dependency closure, model selections, exported declarations, requirement records, the v2 capability report |
| Node | A checked node by `NodeKey` or occurrence key: kind, type, source regions, requirements |
| Outcome | Item, disposition, terminal result, typed cause, catalog code, `Locus` |
| Counterexample | Decoded assignments under the parameter names and the function's `QualifiedName`, with the trusted label where it applies |
| Trace | A finite trace or a lasso: positions, loop start, the state at each position, the deciding position |
| State graph | The states and transitions a simulation or model-check run retained |

**IN-2 Formats.** JSON is the authoritative form: each view and outcome
serializes with a `format` identity. Text and Graphviz DOT are rendered from
the typed value only, so a renderer change leaves the JSON byte-equal.
Renderers feed nothing back into logic (ADR-011 FB-02, FB-09).

**IN-3 Diagnostics and counterexamples.** A text diagnostic reads
`path:line:column: code: message`. Line and column are computed at render
time from the source bytes the caller provides (FR-001). A counterexample
renders as a table of parameter names and values, and a trace as numbered
positions with the loop marked. Rendering follows FR-073: bulk content shows
as a bounded descriptor.

### 10. The first bounded slice

**SL-1** The first slice is the smallest path that crosses the CLI, the
library, a provider and the cache:

- **Library.** The LC-2 request and outcome shape with LC-3 cancellation, for
  `check`, `package`, `execute` (interpreter only, through EB-1) and `prove`.
- **CLI.** The driver binary `quire` with `check`, `compile`, `run`, `prove`
  and `version`, `--format json|text`, the CB-4 exit function and CB-5's
  argument refusal. The `quire` CLI serves these verbs (CB-1).
- **Provider.** The registry with one compile-time provider, the Kani backend
  through CG, and PV-2's front-end and order tests.
- **Cache.** `prove` terminal records only, in a local directory, with CA-2
  to CA-5.
- **Input.** One `1-draft` source with Value-family function claims, with or
  without supplied libraries (FR-099).

The slice leaves plugins, `analyze`, `monitor`, `generate`, AOT, JIT and the
inspection views to later slices.

### 11. Rulings on the draft's questions

The owner ruled on the five questions the draft left open (RU-1 to RU-5) on
2026-10-01, and on RU-6 in review.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | The exit code for O-16 undefined | **Exit 10.** Machine output still labels each item `undefined`; a proof item never settles undefined (the reconciliation References records) | An undefined claim did not hold, which is a logical outcome, and QSpec FR-301 has no separate code. The item label keeps it distinct from a violation | CB-4 |
| RU-2 | Whether `analyze` is in the qualified core | **Inside, through its certificate checkers only**: the zone certificate checker (ADR-026 CF-*), the EN-5 certificate checker (ADR-028 CE-*) and closure checks such as ADR-022's trap re-exploration. The engines stay outside. A `proved` counts only once the in-core checker accepts its certificate | A checker reads only the certificate and recomputes it with its own code, so it is small enough to qualify, and it makes the engine's soundness irrelevant to the verdict | LC-1, CB-2, CB-3, OP-2 |
| RU-3 | Which engines `analyze` covers | **QSL's in-process engines only.** SMT engines are IR's backend and run through the driver | QSL has no SMT code; the SMT backend negotiates and runs like any other provider under `prove` | OP-2 |
| RU-4 | Whether plugins need kernel confinement | **The plugin rights model is deleted.** No rights declarations and no confinement. A plugin's `proved` stays labelled `trusted` | A user installs the plugin, so it is trusted like any program they install. A `proved` from it still cannot be replayed | PL-1, PL-7 |
| RU-5 | Whether plugin results are cached | **No.** | The host cannot show that a third-party run is a function of the cache key | CA-5 |
| RU-6 | How a proof from a native engine with no core certificate checker settles | **`proved`, labelled `uncertified`**, never `inconclusive` | The engine did prove the claim; the label says no core checker confirmed it | OP-2 |

## Consequences

- One binary serves users, and a qualified binary can be cut from the same
  core without touching it.
- Every verb's exit code comes from one function over the O-16 category.
  The scattered mappings go.
- Third-party providers reach the same registry, negotiation and terminal
  records as first-party ones. Their counterexamples count only after
  replay, and their proofs are labelled trusted. Plugins carry no rights
  model: an installed plugin is a trusted program.
- `analyze` proofs count in the qualified core through small certificate
  checkers, so the search engines stay outside it.
- The cache needs no maintenance records. A key is the content's identity,
  so invalidation is automatic.
- AOT and JIT can be added without changing any caller, and replay keeps one
  executor.
- The depth ceiling in `quire-semantic-value` is deleted; node, byte and
  work limits bound checking (ADR-030 RU-1).

## Amendments this record proposes

These amend accepted or proposed records once this record is accepted,
except the ADR-011 amendment, which is applied in place.

- ADR-011 §5: the CLI is the driver's `quire` binary (CB-1). The layer-6 row
  `command < cli < main` keeps `command` as a library and loses `cli` and
  `main` with `quire-spec`.
- ADR-011, applied in place: the S6c native model-checking stage with edges
  E10 and E11 (§1, §2.1, §4), layers A (`qsl-analyze`), P (`qsl-inspect`) and
  X (`qsl-jit`) above the core with `format` moved into layer 1 (§6.1, CB-3,
  OP-1), and extraction row X-12 (§7.3).
- The engine drafts ADR-018 to ADR-028 (EN-1 `model_check` and its
  refinement, reduction, possible-property and hyperproperty extensions,
  EN-4 `statistical`, EN-5, the zone search and the protocol and memory
  engines): every engine sits in layer A, `qsl-analyze`, not in layer 5.
  Its certificate checkers and its settlement map sit in layer 6
  `qsl-replay` (CB-2, OP-2); the engine owns no settlement map. Each engine
  entry takes FR-276's `&Cancel`, not a polling closure.
- ADR-013 T-4: `StageFailure::Cancelled(CancelCause)` and
  `CallFailure::Cancelled` (LC-3).
- ADR-012 §7.2: CG's closed backend kind gains the process-provider variant,
  and CG settles a descriptor whose `ProviderOrigin` is `Process` to it
  (PV-4). `BackendDescriptor` gains the origin (PV-1, FR-288).
- `quire-semantic-value` `checking`: `MAX_CHECKING_DEPTH` and
  `DepthAboveMaximum` are deleted, with no depth setting in their place
  (LC-4, ADR-030 RU-1).
- QSpec FR-306: its JIT and AOT rows follow EB-4, and the cache key follows
  CA-2. QSpec FR-305: the manifest binding follows PL-4 and PL-5, and its
  effects and rights members and rights-mismatch refusal are removed
  (RU-4).

## Alternatives Considered

- **Keep `quire-spec` as a second user CLI for the QSL stages.** Rejected.
  Two user surfaces would duplicate verbs and exit mappings, and only the
  driver sees every stage.
- **Put the CLI in QSL and call CG from it.** Rejected. CG depends on QSL, so
  a QSL binary that calls CG closes a package cycle (ADR-011 FB-11).
- **Dynamic-library plugins.** Rejected by QSpec FR-305, which defines no
  dynamic-library ABI. The out-of-process wire keeps a plugin crash out of
  the host's address space.
- **A rights model for plugins.** Rejected (RU-4). The user installs the
  plugin, so it is trusted like any installed program.
- **The whole `analyze` engines in the qualified core.** Rejected (RU-2). A
  certificate checker is far smaller than the search it checks, so it is the
  part worth qualifying.
- **Compile-time plugins only.** Rejected. A third party would have to
  rebuild the host to add a provider.
- **Invalidate cache entries by tracking file and tool versions.** Rejected.
  A content key misses on every meaning change by construction, and a
  version ledger adds records that can drift.
- **Cache every stage.** Rejected. Parsing, checking and execution are cheap
  and deterministic, so caching them adds a failure mode and saves little.
- **Let the JIT or AOT backend fall back to the interpreter silently.**
  Rejected. QSpec FR-301 forbids fallback targets, and a silent switch hides
  which engine ran. The caller chooses.
- **Replay through the JIT for speed.** Rejected. Replay defines agreement
  with the reference semantics, so it runs on S6a (ADR-011 FB-07).

## References

- Owning ticket: QSL-390 (ARCH-50). This record also covers QSL-392 (ARCH-52,
  the first bounded slice, §10) and QSL-393 (V1-A06, V1-A06a: the typed
  library operations, provider negotiation that leaves results unchanged,
  the plugin wire and checked-only AOT and JIT).
- Implementation tickets it unblocks: QSL-14, QSL-10, QSL-66, QSL-89.
- The JIT code-generator benchmark (Cranelift or LLVM): RES-56.
- Version and provenance records in QSpec FR-301, FR-305 and FR-306 that
  this design does not use: QSL-395.
- Owner rulings of 2026-10-01: recorded on QSL-390.
- Ruling RU-6 (`uncertified`) and the in-core checker list: recorded on
  QSL-390. The checkers' defining drafts are PR #562 (EN-1 closure, SCC and
  ranking), PR #564 (simulation relations), PR #571 (product closure),
  PR #573 (zone) and PR #579 (EN-5).
- Implementation follow-ups this record states for code: one
  `qsl-foundation` `BackendId` replacing `qsl_route::BackendId` and
  `qsl_replay::identity::Backend` (FR-288); `ProofCategory` folded into `Category` (FR-285); `format` moved
  from root `src/format.rs` to `qsl_cst::format` (X-12).
- Team-leader decision reconciling RU-1 with the QSL-366 ruling (an undefined
  claim evaluation settles `refuted` with cause `UndefinedEvaluation{where,
  cause}`), made on 2026-10-01 in answer to SR-1060 FND-002: proof items (`prove`,
  `analyze`) never carry the `undefined` label, and an undefined evaluation
  there is `refuted`, category violation, exit 10. The `undefined` label
  stays only on non-proof evaluation items, as FR-109 gives it, and exits 10,
  so RU-1's exit holds for both. An undefined clause evaluation over a
  supplied trace (`monitor`) is a violation, exit 10.
- Draft records cited: ADR-018 (temporal properties, EN-1), ADR-022
  (possible properties, trap re-exploration), ADR-024 (statistical and
  probabilistic properties, EN-4), ADR-026 (dense time, CF-*) and ADR-028
  (exact probabilistic engine, EN-5, CE-*).

---
id: ADR-030
title: "Arbitrary nesting depth: resource limits only, no fixed depth caps"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# ADR-030: Arbitrary nesting depth: resource limits only, no fixed depth caps

## Status

Draft, 2026-10-01. Design only: this record makes the decisions. The QSL
requirements that carry them are FR-255 to FR-264 and FR-356 (US-027, TC-720
to TC-739 and TC-902; the walker toolkit's own TC-898 and TC-899 are in
`agent-ix/quire-walk`), with the D-5 deletions applied in place. The QSpec requirements for
D-1 and D-2 follow as separate work. No code changes
with this record. The owner's rulings on the draft's questions are
recorded in D-9.

## Context

The owner ruled that deep nesting must be supported, and that a known depth
limit is answered by a redesign, not by a cap.

QSL expressions are binary trees. A long sum, a long `else if` chain or a long
run of nested `let`s is as deep as it is long. Today two fixed constants
refuse such expressions well before any size limit is reached: the forms
stage's `DEFAULT_FORMS_NESTING_DEPTH` (`qsl-forms/src/dispatch.rs`) and the
checker's `MAX_CHECKING_DEPTH` (`quire-semantic-value/src/checking.rs`, which
`CheckingLimits::new` refuses to exceed). NFR-011 makes the checker's constant
a requirement. QSpec FR-146 states that Complete V1 defines no expression depth
or nesting limit for checking, and that a host stack limit is never a
Complete-V1 outcome.

Other fixed depth constants sit under QSL's identity and intake paths:
`quire_canonical::Limits::MAX_DEPTH`, which is the stack budget of a
serde-driven recursive encoder; `IDENTITY_LIMITS`
(`quire-semantic-value/src/semantic_node.rs`), which takes that constant as
its depth; and the semantic-IR reader's `json::MAX_DEPTH`, which QSL's intake
mirrors (`qsl-semantics/src/model/intake.rs`). Several walks recurse natively
in proportion to input, and are safe today only because a cap runs first:
the checker's text-leaf walk (`LeafWalk::walk`, `check/lowering.rs`),
`node_key::term`, the forms `control_anchors` walk, and the derived `Clone`,
`Debug`, `PartialEq` and `Drop` of `qsl_forms::Expression` (whose `Drop` is
already iterative), the checked `Node` and `quire_exact::ValueType`.

Several pieces are already iterative and are the pattern this record extends:
the CST lexer and parser keep explicit stacks; forms builds `Expression` trees
over an explicit stack; typing, facts and expression lowering in the checker
run over explicit frame stacks; the evaluator runs on an explicit task stack;
`quire_exact::Value` has hand-written iterative traits; and the
`quire-canonical` encoder already keeps an explicit frame stack for open
objects. QSL's emitted v2 checked package is already one node per expression,
with arguments as references, but the v2 schema still admits inline nested
terms.

## Decision

Item ids `D-` are local to this record. Other artifacts cite them as
`ADR-030 D-n`.

### D-1. The ecosystem depth rule (QSpec FR-460)

QSpec states one rule for every stage of every implementation: parser,
reader, encoder, checker, lowering, generator, evaluator, replay and every
wire format.

1. **Depth is not a limit.** Nesting depth is bounded only by the caller's
   resource limits: input bytes, node and edge counts, work units and time.
   No stage has a depth limit kind, a caller depth setting, a compile-time
   depth ceiling, or a constant sized to a stack (RU-1).
2. **No walk uses native recursion proportional to input.** Every structure
   walk, including the derived `Clone`, `PartialEq`, `Hash`, `Debug` and
   `Drop` of a recursive type, is iterative by default, in one of two ways
   (FR-356):
   - **Arena order.** An arena stores each node after its children, so a
     bottom-up computation is one forward loop over the arena with no stack.
   - **The walker toolkit.** A top-down or mutually recursive walk runs on
     the one shared `no_std` walker toolkit, the crate `quire-walk`, in its own repository
     `agent-ix/quire-walk`: an
     explicit heap stack of typed frames, with enter and exit callbacks. The
     toolkit is Kani-verified. It is a shared leaf in FB-05's class (ADR-011
     §6.1 layer W), so QSL, IR, CG and RT all use it.

   A representation whose depth its schema fixes needs neither, and
   `quire-exact`, a leaf with no dependency, keeps hand-written iterative
   traits (D-4.7).
3. **Heap stacks are charged.** Every heap stack, and every recursion through
   the `maybe_grow` wrapper (item 6), grows by at most a constant per node
   already charged against a node, byte or work limit, so the configured
   limits bound memory.
4. **Reaching a limit is a stated outcome.** A stage limit settles as
   `stage_limit_exceeded` (ADR-014 B-3, FR-096), and an execution budget as
   `incomplete { limit_kind, ... }` at the denied charge (ADR-014 B-2). Each
   names the limit, its value and how to raise it (D-3). A host stack limit
   is never an outcome, because no stage reaches one.
5. **Every limit has a default.** A limit the caller does not configure takes
   its published default (ADR-014 §2, "inherited"). Defaults are size and work
   values sized for realistic specifications, never depth values. A deep
   input that does not fit them runs under limits the caller raises to fit.
6. **The qualified core is iterative only.** In the qualified core (the S0 to
   S4 checker, the prove path and the certificate checkers; ADR-029 CB-2) no
   walk grows or switches the native stack. Outside the core, a walk whose
   conversion to the toolkit is awkward may grow the stack on demand as a
   justified exception, only through QSL's one `maybe_grow` wrapper. The
   wrapper is the std-only crate `qsl-walk-grow` (ADR-011 layer WG), outside
   the core, which no core crate depends on; `quire-walk` stays `no_std`
   with no features, so no feature unification can bring `stacker` into the
   core. The wrapper is a plain call under `cfg(kani)`. Each use has a test
   at 100,000 depth on a thread with a small fixed stack.
7. **Every public core entry point is tested deep.** Each public entry point
   of the qualified core has a test that drives a 100,000-deep input through
   it on a thread with a small fixed stack, so leftover or hidden recursion
   fails at test time (D-6).

This generalises FR-146's checking-only statement to every stage.

### D-2. Flat v2 body sub-positions (QSpec FR-322 body grammar)

QSpec FR-322 defines a v2 node `body` by a stratified grammar in which no
production names itself or a higher stratum. The JSON depth of every v2
package is then fixed by the schema, whatever the model's size.

| Stratum | Productions |
| --- | --- |
| Leaf | `literal`, `reference`, `dependency_reference` |
| Group | `aggregate` whose members are each a Leaf or a `binding` of a Leaf |
| Tuple | `aggregate` whose members are each a Leaf or a Group |
| Member | Leaf, Group, or `binding` of a Leaf, a Group or a Tuple |
| Body | Leaf, `application` whose `arguments` are each a Member, `aggregate` whose members are each a Member, `frame`, or `abstraction_relation` |

Each composite subterm is its own node, reached by `reference`. A reader
refuses a body outside the grammar as a malformed wire (`refused`), not as a
limit.

QSpec owns the grammar, and QSL cites it. A `binding` never holds a `binding`.
Expression results and operands are references or literals, an application
argument is a Member (a reference, a literal, a group of bindings or a
binding of a Leaf, a Group or a Tuple), function nodes are an aggregate of
bindings whose `parameters` value is a group of references, and unit-power
members are groups of bindings. The strata are closed. If an emitted shape
falls outside them, lowering splits it into nodes rather than adding a
stratum. Two QSL shapes were a binding of a binding: an optional record
field (FR-092) and a fold's step (FR-093). Each is now a binding of a Group
that holds the inner binding, so those nodes' ids differ from the
four-stratum form's, and so do the ids of every node that names them.

With D-2 the reader-declared maximum depth for FR-322 is unnecessary: the
schema fixes depth, and bytes, nodes, edges and work bound the rest.

### D-3. Limit outcomes and diagnostics

A limit outcome tells the caller everything needed to run the input again
under a larger limit.

1. **The outcome names three things.** These are:
   - the limit: its stage and kind, for example S3 nodes;
   - its value: the configured bound in force, and the count the refused
     charge would have reached;
   - its setting: the name by which a caller raises it.

   The locus stays as FR-096 defines it.
2. **One setting name per limit.** Every configurable limit has one stable
   dotted name built from its stage and its counter, for example `s1.tokens`,
   `s3.nodes`, `s3.work_units` or `intake.input_bytes`. The same name is used
   at every entry point:
   - the library limits type, through its builder for that field;
   - the replay request, as the `stage_limits` entry of that name;
   - the CLI, as `--limit <name>=<value>`. The user CLI is the driver's
     (`quire`, ADR-029 CB-1), and the driver CLI exposes the caller limits.
     QSL provides the library operation that applies a `<name>=<value>`
     setting to the stages' limits, which the driver CLI calls.
3. **Every named limit can be raised at every entry point.** Replay carries
   every stage limit in its request and passes each one through. Today it
   applies the proving run's S1 byte limit and S3 work budget, and runs every
   other stage at its defaults. QSL's settings operation accepts every setting
   name, so the driver CLI accepts every setting name.
4. **The diagnostic type carries the setting.** `qsl_foundation`'s
   `LimitExceeded` carries the setting with the kind, configured bound,
   actual count and locus. Each stage's limits type maps each of its fields
   to its setting in one place.
5. **Rendering.** The rendered diagnostic states the limit, the configured
   value, the count reached, the locus and the setting. For example:
   "S3 node limit 100000 reached (100001) at `f`; raise it with
   `--limit s3.nodes=<n>` or the request's `stage_limits` entry `s3.nodes`".
6. **Catalogs.** The `stage_limit_exceeded` row of the native diagnostics
   catalog carries the setting name with the limit kind and bound (overlap
   item O-5). An `incomplete` outcome names its accounting counter, which is
   already its setting under `quire.value.accounting/v1`.

### D-4. Components in QSL's lane

For each component: the redesign, the limit that bounds it, and the
deep-input criterion as a design statement. Every limit is reported as D-3
states. Every criterion has two halves: any input that fits the default
limits succeeds whatever its depth, and a deep input that does not fit them
succeeds once the caller raises the limits to fit. The 100,000-deep cases
below set their own limits, and each runs on a thread whose stack is far
smaller than one frame per level would need (D-6).

#### D-4.1 CST lexer and parser

- **Redesign.** Already iterative: the lexer's delimiter stack and the
  parser's explicit stack and memo table. The bracket-nesting ceiling
  (`qsl_cst::Limits::nesting`) is deleted. At any position the bracket depth
  is fixed by the input, so the memo table stays bounded by the production
  count per position without it.
- **Limit.** Source bytes, tokens, syntax nodes and the parser's work budget
  (NFR-001), each a `stage_limit_exceeded` outcome (D-3).
- **Criterion.** A 100,000-deep bracket nest, `not` chain, sum, `else if`
  chain and `let` chain each parse under token and node limits raised to fit
  them.

#### D-4.2 Forms (S2)

- **Redesign.**
  - `Expression` becomes an arena: one `Vec` of nodes per declaration body,
    children named by a typed index (`ExprId`). Derived `Clone`, `PartialEq`,
    `Debug` and `Drop` then run over a flat vector. `Debug` renders the tree
    from an explicit stack. The hand-written iterative `Drop` goes away with
    the boxes.
  - `control_anchors` becomes a loop over an explicit stack of
    `(control node, child cursor, scope length on entry)` frames. Leaving a
    frame truncates `scope` to the recorded length, which replaces the
    recursive `with_scope` push and pop.
  - `DEFAULT_FORMS_NESTING_DEPTH` and `FormsLimits::nesting_depth` are
    deleted. With no other field, `FormsLimits` is deleted with them.
- **Limit.** S2 builds at most one form node per CST node, so S1's node limit
  bounds it. S2 adds no limit of its own.
- **Criterion.** Every 100,000-deep S1 input in D-4.1 builds its forms, and a
  100,000-deep `await ... then` and `repeat ... exhausted` control chain
  builds its anchors.

#### D-4.3 Checker (S3)

- **Redesign.**
  - `MAX_CHECKING_DEPTH`, `CheckingLimits`' depth, `DepthAboveMaximum` and
    `CheckingLimitKind::Depth` are deleted. `CheckingLimits::new` takes no
    depth and cannot fail.
  - The checked `Node` becomes an arena, as in D-4.2 (`NodeId` into one
    vector per checked body), so its derived traits are flat.
    - `LeafWalk::walk` runs on the walker toolkit beside its existing `stack`
    of path segments, with one typed frame per entered type: the `ValueType`
    being walked and, for a composite, its field cursor and the
    `open`-list length on entry. `open` and `prefixes` keep their roles.
  - QSL's `SemanticTerm` becomes stratified Rust types matching D-2
    (`LeafTerm`, `GroupTerm`, `TupleTerm`, `MemberTerm`, `BodyTerm`). `node_key::term`
    becomes a fixed-depth match, and its depth guard is deleted. Body
    preimages then have a schema-fixed depth.
  - Every other walk over a checked `Node` that recurses (for example the
    facts helpers that follow a field-access chain) runs in arena order or
    on the walker toolkit (D-1 item 2). The checker is in the qualified core,
    so none of its walks grows the stack on demand (D-1 item 6).
- **Limit.** The checking node limit, the per-declaration preimage byte limit
  and the work budget (NFR-011), each a `stage_limit_exceeded` outcome
  (D-3). A text-leaf walk charges one node unit per leaf and one work
  unit per composite entered, as today; each walk frame is charged by the
  composite it enters.
- **Criterion.** A 100,000-deep sum, `else if` chain and `let` chain check,
  lower and emit under node and work limits raised to fit them; a
  100,000-deep `Option` type and a 100,000-record composite chain lower their
  text leaves.

#### D-4.4 `quire-semantic-value` `IDENTITY_LIMITS`

- **Redesign.** `IDENTITY_LIMITS` becomes the published default (16777216
  bytes) of the caller's `identity.input_bytes` limit (FR-255), a byte limit
  the caller raises with no ceiling, and it loses its depth.
  Every identity preimage it encodes has a schema-fixed depth after D-2 and
  D-4.3: node keys, compound-unit ids, the checked and v2 package identity,
  type and enum preimages. Each such type encodes through `quire-canonical`'s
  fixed-depth serde path (D-4.5). An identity over data whose depth follows
  the input (a simulation state key, a replay value, an intake or
  observation document) encodes through the event API (D-4.5) instead.
- **Limit.** The caller's byte budget for the identity's input, already
  charged by the stage that produced it; a site with a tighter byte budget
  passes its own `Limits`. An encoding past the byte ceiling is reported by
  the calling stage as its byte limit (`stage_limit_exceeded` or
  `incomplete`), never as a malformed value.
- **Criterion.** The identity of a package holding a 100,000-deep expression
  is minted, and equals the identity minted on an ordinary thread.

#### D-4.5 What QSL relies on from `quire-canonical`

`quire-canonical` is a separate repository and designs its own crate (D-8
overlap item O-6). QSL relies on these capabilities, named here, not
designed:

- canonical RFC 8785 encoding bounded by bytes only, with no depth limit;
- encoding of data whose depth follows the input, driven from the caller's
  own explicit stack (the event API);
- a reader of untrusted JSON into a tree whose traits do not recurse,
  refusing malformed input with its byte offset, bounded by an input byte
  limit;
- a serde encoding path that only fixed-depth types can take.

QSL's own behaviour over them: QSL reports the crate's byte error as the
calling stage's byte limit (D-3), maps the crate's malformed-input refusal to
the calling site's malformed-input refusal, and routes each identity to the
fixed-depth path or the event API as D-4.4 states.

#### D-4.6 Semantic-IR intake

- **Redesign.**
  - Intake reads the document once, through `quire-canonical`'s reader
    (D-4.5), under the caller's intake byte limit.
  - `ByteScan` and `IntakeLimit::NestingDepth` are deleted. The reader
    refuses a lone surrogate escape itself.
  - The derived view (`PackageDocument::tree`) is the reader's arena tree,
    in place of a `serde_json::Value`, whose own `Drop` recurses.
  - The `sha256-jcs` digest encodes that tree through the event API.
  - The semantic-IR crate's checks take the same tree (overlap item O-4), so
    the document is parsed once. QSL's lock on that crate moves to the
    revision that does this.
- **Limit.** The intake byte limit, caller-configurable with a published
  default, refused as `resource_exhausted`/`intake-limit-exceeded` with the
  limit, its value and its setting (D-3). Tree size, and so memory, is linear in bytes.
- **Criterion.** A 100,000-deep package document is admitted or refused on
  its content, never on depth. Composite cycles of 300 and of 100,000 types
  are reported as cycles.

#### D-4.7 Evaluator

- **Redesign.** Expression evaluation and calls already run on an explicit
  task stack. What remains is the types around it:
  - `quire_exact::ValueType` gets hand-written iterative `Clone`,
    `PartialEq`, `Debug` and `Drop`, following `Value`'s existing pattern.
    These types also build for `no_std` targets, where a deep native
    recursion fails silently rather than aborting.
  - The simulation state key (`qsl-eval/src/simulation/key.rs`) encodes
    through the event API. `TransitionSystem::Key` and `TransitionId` change
    their bound from `Serialize` to an event-source trait that writes into
    the canonical writer from an explicit stack. `quire_exact::Value`
    implements it iteratively, and a fixed-shape key implements it through
    the fixed-depth serde path.
- **Limit.** `work_units` fuel under `quire.value.accounting/v1`: one
  `function.call` charge per call, with no call-depth counter (FR-146). A
  denied charge is `incomplete { limit_kind: work_units, ... }`. Simulation
  keeps its explicit exploration limits.
- **Criterion.** A recursive function called 100,000 deep evaluates within a
  work budget sized for it. A 100,000-long recursive list value evaluates,
  keys as simulation state, compares, clones and drops.

#### D-4.8 Replay

- **Redesign.** Replay inherits every change above: recompilation through
  D-4.1 to D-4.3, identities through D-4.4, values through D-4.7. Canonical
  assignment values decode into `quire_exact::Value` from `quire-canonical`'s
  reader tree (D-4.5). Every identity and digest over a value encodes
  through the event API.
- **Limit.** The proving run's stage limits and accounting limits, carried in
  the request, every stage limit passed through (D-3), and the request's
  encoded-byte bound. Each is reported as the replay outcome FR-098 defines
  for it, with the limit, its value and its setting.
- **Criterion.** A replay request whose source holds a 100,000-deep
  expression, and whose assignment carries a 100,000-deep value, replays to
  the same verdict as the proving run.

#### D-4.9 v2 wire: emitter and reader

- **Redesign.**
  - The emitter produces D-2's form. A conformance test asserts that every
    emitted body is in the stratified grammar.
  - The reader (`qsl-package/src/checked_v2.rs`) deletes
    `V2ReadLimits::depth` and its clamp to IR's reader maximum once IR's
    reader enforces D-2 (overlap item O-1). The decoded nodes are then
    fixed-depth types, so the derived traits on `V2Read` stop recursing in
    proportion to input.
- **Limit.** Artifact bytes, nodes, edges, occurrences, diagnostics and work,
  passed to IR's reader as today and reported as its limit with the setting
  that raises it (D-3). A
  nested inline term is a malformed wire.
- **Criterion.** A package holding a 100,000-deep expression is emitted, read
  back and verified. A wire with a nested inline term is refused as
  malformed.

#### D-4.10 Other JSON reads in QSL

- **Redesign.** Every QSL read of untrusted JSON that does not go through a
  typed reader uses `quire-canonical`'s reader and its arena tree (D-4.5):
  - library package identity (`library/package_identity.rs`);
  - observation digest admission (`model/observation.rs`);
  - the observation document reader (`model/observation/document.rs`).
  
  A parse failure becomes a malformed-input refusal and an over-limit input a
  limit refusal, each reported as what it is. Today these sites map
  `serde_json`'s own depth refusal to "not an object" or to "no digest". The
  observation document reader's nesting-depth limit is deleted.
- **Limit.** The calling stage's input byte limit.
- **Criterion.** Each site reads a 100,000-deep document and reports it on
  its content.

### D-5. QSL requirements this record changes

Recorded here as decisions. The documents themselves are amended with the
requirements work that follows this record.

- **NFR-011's depth rule is deleted.** The nesting-depth ceiling, its default,
  its maximum, its metric row, the "Nesting level" counter and its default
  derivation go. The node, preimage-byte and work ceilings stay. FR-062's
  depth refusal criterion goes with it.
- **NFR-001's bracket-nesting ceiling is deleted** (D-4.1). The token, node,
  byte and work ceilings stay.
- **FR-091's S2 depth limit is deleted** (D-4.2). S2 is bounded by S1's node
  limit.
- **FR-093's text-leaf walk** is bounded by the node and work ceilings only
  (D-4.3).
- **The library resolution depth is deleted.** `DependencyLimits` bounds the
  import graph by library, import-edge and source-byte limits (FR-099), and
  `PackageLimits` carries no depth (FR-111, FR-087).
- **ADR-011 and ADR-012 are amended in place.** Their rows that bound a body,
  a `case` or a node key by `MAX_CHECKING_DEPTH`, `DepthAboveMaximum` or a
  nesting-depth limit kind now bound it by the node, preimage-byte and work
  limits.
- **No native limit is clamped and none bounds depth.** NFR-007, NFR-008 and
  the native-profile requirements (FR-002, FR-003, FR-009, FR-013, FR-015,
  FR-016, FR-024, FR-025, FR-033, FR-034, FR-036, FR-040, FR-041, FR-042,
  FR-051, FR-052) take each caller limit as
  given, with a published default the caller may raise, and bound nesting by
  bytes, nodes, entries and work.

### D-6. Verification design

- Each criterion in D-4 is one test on a thread with a small fixed stack
  (`qsl-cst`'s parser test is the existing example), so a pass shows that
  stack use does not grow with depth.
- Each public entry point of the qualified core, and each `maybe_grow` call
  site outside it, has a test at 100,000 depth on a thread with a small
  fixed stack (D-1 items 6 and 7; FR-356, TC-902). The walker toolkit has
  its own small-stack tests and Kani harnesses (TC-898, TC-899, in
  `agent-ix/quire-walk`).
- Deep inputs cover a sum, an `else if` chain and nested `let`s through parse,
  forms, check, lower, emit, v2 read and evaluate; a deep `Option` type and a
  recursive list value through state key, replay and identity; deep JSON
  through intake, each D-4.10 site and `quire-canonical`; and long composite
  cycles through intake.
- Existing limit tests stay, re-parameterised onto the remaining limits.
- **Default limits.** The defaults stay as they are, sized for realistic
  specifications. Any input that fits them succeeds whatever its depth,
  including a chain as deep as the default token limit allows. A deeper
  input needs its limits raised to fit; the 100,000-deep tests set their own
  limits.
- **Limit outcomes.** For each stage, one test drives a deep input past a
  default limit and asserts the outcome's limit, value and setting (D-3).
  It then reruns the input with that setting raised, through the library,
  the replay request and the CLI, and asserts that it succeeds.

### D-7. QSL's slices and what they wait on

QSL's own work lands in four slices. Each names only the other lane's work
it waits on.

- **Slice 1. Waits on nothing outside QSL.**
  - The walker toolkit `quire-walk` (its own repository,
    `agent-ix/quire-walk`) and the `maybe_grow` wrapper crate
    `qsl-walk-grow` (FR-356), landing first,
    since the walks below run on it.
  - Forms arena and iterative `control_anchors` (D-4.2).
  - Checked `Node` arena, and `LeafWalk` and the remaining checker walks on
    the walker toolkit
    (D-4.3).
  - Stratified `SemanticTerm` and fixed-depth `node_key::term` (D-4.3).
  - Iterative `ValueType` traits (D-4.7).
  - Deletion of `DEFAULT_FORMS_NESTING_DEPTH`, `MAX_CHECKING_DEPTH` and the CST
    nesting ceiling.
  - Deletion of every depth limit kind: `qsl_foundation`'s
    `LimitKind::NestingDepth` and its `nesting-depth-exceeded` cause,
    `SyntaxLimit::NestingDepth`, `CheckingLimitKind::Depth`, the checked
    family's `StageLimits` nesting depth, and the library resolution depth,
    whose import graph node, edge and byte limits replace it (FR-096,
    FR-099, FR-111).

  The precondition is that every identity preimage a checked body produces
  is fixed-depth (D-4.4). The slice confirms it with a deep-expression
  identity test before deleting the checker cap.
- **Slice 2. Waits on the `quire-canonical` capabilities (O-6).**
  - `IDENTITY_LIMITS` loses its depth, and every identity preimage takes the
    fixed-depth path (D-4.4).
  - The state key moves to the event API (D-4.7).
  - Replay value decoding and digests (D-4.8).
  - The three D-4.10 sites move onto the shared reader.
  - The D-3 setting names, the `LimitExceeded` setting field, replay's
    pass-through of every stage limit, and the settings operation the driver
    CLI's `--limit` calls (ADR-029 CB-1).
- **Slice 3. Waits on the semantic-IR crate taking the shared tree (O-4).**
  Intake on the shared reader, and the lock bump (D-4.6).
- **Slice 4. Waits on IR enforcing D-2 (O-1).** `V2ReadLimits::depth` is
  deleted (D-4.9).

### D-8. Overlap items with other lanes

Named here, not designed. Each owner designs its own internals to D-1 and
D-2.

- **O-1 IR.** The checked-package reader's maximum depth and the stack
  machinery behind it are IR's to remove. So are enforcing D-2 in term
  validation, the reader's recursive decode, admit and preimage, the
  quadratic reference grouping, and moving its digest onto
  `quire-canonical`. IR's reader reads through `quire-canonical`'s shared
  reader. QSL's D-4.9 waits on this.
- **O-2 CG.** Generated-code analysis and rendering, and generated Rust whose
  nesting follows expression depth, are CG's. One option is flat, one
  binding per node.
- **O-3 RT.** `MAX_CALL_DEPTH` and RT's host-recursive call path are RT's to
  replace with work fuel and an explicit frame stack. So are RT's recursive
  `ValueType` and `CollectionType`, and moving its snapshot reader onto
  `quire-canonical`'s shared reader. RT's own checking
  limits are a call-depth limit with defaults different from QSL's.
- **O-4 Filament (semantic-IR in `filament-core-data`).** This covers:
  - the JSON reader, with its depth constant and fixed byte constant. It is
    replaced by `quire-canonical`'s shared reader, under the caller's byte
    limit, and the crate's checks take the reader's arena tree;
  - the recursive `Json` traits and writers, which the arena tree and the
    `quire-canonical` encoder replace;
  - the rules' traversal constant `DEPTH_LIMIT`.

  **Likely bug.** `composite_visit` and `package_visit` return "no cycle"
  once their depth passes `DEPTH_LIMIT`, so a composite cycle longer than
  that constant goes unreported. The cause is traced in the code, not shown
  by a test. Reporting every composite cycle, whatever its length, is a
  Filament-owned requirement; Filament designs the search. QSL's D-4.6
  criterion tests the outcome.
- **O-5 QSpec.** D-1 and D-2 as requirements, including FR-322's read limits,
  the depth rows of the diagnostics catalogs that D-1 retires, and the
  setting name on the `stage_limit_exceeded` row (D-3).
- **O-6 `quire-canonical`.** The capabilities D-4.5 names (byte-only
  canonical encoding, an event API driven from the caller's stack, the
  shared non-recursive JSON reader, and a serde path for fixed-depth types
  only) are `quire-canonical`'s to design and deliver, a `quire-canonical`
  follow-up. QSL's slice 2 waits on them.

### D-9. Rulings on the draft's questions

The owner ruled on the questions the draft left open, on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Whether any stage keeps an optional caller depth setting | **No depth setting of any kind.** | Depth is not a resource once nothing recurses. Nodes, bytes and work bound memory and time | D-1 items 1 and 5; D-4.1 to D-4.3, D-4.6, D-4.10; Alternatives |
| RU-2 | Where the shared iterative JSON reader lives | **In `quire-canonical`.** | It is the shared `no_std` leaf crate, so one reader serves QSL, IR and RT | D-4.5; D-4.6, D-4.8, D-4.10; D-7 slice 2; O-1, O-3, O-4, O-6 |
| RU-3 | Whether the default size limits rise to fit 100,000-deep inputs | **The defaults stay, sized for realistic specifications.** A limit reached gives `stage_limit_exceeded` naming the limit, its value and how to raise it. The 100,000-deep criteria set their own limits | A default bounds a realistic specification's cost. A deep input states the limits it needs, and the outcome tells the caller which setting to raise | D-1 items 4 and 5; D-3; D-4 criteria; D-6 |

## Consequences

- Expressions, types, values and documents nest to any depth their size
  limits admit. The limit a user meets is the one that measures what the
  input costs: bytes, nodes or work.
- Depth stops being a limit kind in QSL. Removing `LimitKind::NestingDepth`,
  `CheckingLimitKind::Depth`, `IntakeLimit::NestingDepth` and the S1 and S2
  nesting kinds removes them from the effective limits a checked package
  records.
- `qsl_forms::Expression`, the checked `Node` and `SemanticTerm` change shape.
  Every consumer that matches on them moves to arena ids or stratified
  variants in the same slice.
- A recursive type has no serde path into the canonical encoder, so a QSL
  caller that serialises one fails to compile rather than at run time.
- The v2 wire's depth is fixed by its schema, so readers of it need no depth
  defence.
- One JSON reader and one canonical encoder serve QSL, IR, RT and the
  semantic-IR crate, so all of them read the same document the same way.
- A caller who meets a limit learns from the outcome which setting to raise,
  and can raise it from the library, the replay request or the CLI.

## Alternatives Considered

- **Raise each cap.** Rejected. A higher constant moves the failure point;
  the owner ruled that a known limit is a redesign.
- **Run deep work on a fixed large-stack thread.** Rejected. It moves the
  constant to the thread size. A Rust stack overflow aborts and cannot be
  caught, and `no_std` targets have no such thread.
- **Grow the stack on demand.** Rejected in the qualified core. Growing the
  stack switches stacks with `unsafe` code that Kani cannot verify, so a
  core walk that relied on it would leave the core's proofs. Outside the
  core it is a justified exception where conversion to the walker toolkit
  is awkward, through the one `maybe_grow` wrapper and a 100,000-depth test
  for each use (D-1 item 6). A grown stack has no fixed size, so it is not
  the fixed-thread alternative above.
- **Keep an optional caller depth knob.** Rejected (RU-1). Once every walk
  is iterative, memory and time are linear in nodes and bytes, so a depth
  knob bounds nothing those limits do not. It would also keep depth as a
  limit kind in every catalog.
- **A JSON reader per crate, or one in a QSL foundation crate.** Rejected
  (RU-2). IR and RT need the same reader and cannot depend on QSL.
  `quire-canonical` is the shared `no_std` leaf they all depend on already.
- **Raise the default size limits so a 100,000-deep chain fits unconfigured.**
  Rejected (RU-3). Defaults are sized for realistic specifications. A deep
  input raises its limits, and the outcome says which setting to raise.
- **Hand-written iterative traits instead of arenas for `Expression` and the
  checked `Node`.** Rejected for those two types. An arena fixes every
  derived trait at once and keeps them correct as variants are added.
  Accepted for `ValueType`, which is a kernel type shared with `no_std`
  builds, where `Value` already sets the hand-written pattern.
- **A non-recursive serde serializer.** Not possible. `Serialize` is visitor
  recursion over the value, so a recursive type always recurses through
  serde. Hence the fixed-depth serde path and the event API.
- **A reader-declared maximum depth for the v2 wire.** Rejected. D-2 fixes
  the depth by schema, so there is nothing for a reader to declare.

## References

- Linear QSL-381 (this redesign), with the owner rulings and the ecosystem
  research it carries.
- The QSpec half (Linear STD-143, which supersedes STD-125): D-1 is QSpec
  FR-460; D-2 is FR-322's body grammar, AC-39 to AC-42; D-3's setting names
  are FR-461 and the FR-323 `stage_limits` entries.
- Linear IR-495 (IR), IR-496 (CG), IR-497 (RT): O-1 to O-3.
- QSpec FR-146 (no checking depth limit; no host stack outcome) and FR-322
  (checked-package artifact).
- ADR-011 §2.3 (stage limits), ADR-013 §2 (one RFC 8785 implementation),
  ADR-014 §1 and §2 (bound taxonomy; absent bounds are inherited).

---
id: ADR-017
title: "Protocol/frame, refinement and abstraction-relation boundaries (ARCH-43)"
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
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-003
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-010
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-012
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-177
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-196
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-323
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-331
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-350
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
---
# ADR-017: Protocol/frame, refinement and abstraction-relation boundaries (ARCH-43)

## Status

Proposed, 2026-09-29. Owning ticket: GitHub #223 (ARCH-43),
epic QSL-34 (#205), Layer 4 (coordination and conformance architecture). It
is a prerequisite of gate QSL-15 (#224, ARCH-G4) for that gate's scenarios 5
(scoped protocol/frame clause), 7 (spec/profile refinement regression) and 8
(model-to-implementation relation with an unbound refusal). It implements no
feature. Supersedes nothing. Its `/spec-review all` is SR-800 to SR-807 in
[`spec/reviews/arch-43-boundaries/`](../reviews/arch-43-boundaries/integrity.md).

Amended 2026-10-01 (abstraction relation FR pass, FR-304 to FR-307): AR-1,
AR-2, AR-4 and §5 follow QSpec's answers to Q-1, Q-2 and Q-7, which are
decided: QSpec FR-450 spells the declaration (Q-1), QSpec FR-451 spells the
v2 node (Q-2), and QSpec FR-353-AC-1 accepts the derived operation key
(Q-7). AR-2's frame binding receiver is a `RustReceiver`.

A bare FR, AD or TC id is a QSL artifact; a QSpec artifact is always written
"QSpec FR-nnn". Item ids `PF-`, `RF-`, `AR-`, `G-`, `Q-` and `TK-` are local
to this record; other artifacts cite them as `ADR-017 PF-n`.

## Context

Measured on QSL `main` at `99e9b6c7` and QSpec `main` at `4634f5fd`.
External ticket text is quoted as data, not verified in its repository.

**Layer 3 decisions this record consumes, not reopens.** ADR-011 (stage DAG,
E1 to E9, forbidden bypasses, lanes M-6c to M-6e), ADR-012 (families, the
`ProtocolClause` and `Relation` rows, §12.2 frame change set, §15 state
clauses), ADR-013 (identity owners O-01 to O-27, conversions C-01 to C-30,
the R-06 name rule), ADR-014 (extent, bounds, outcome table) and ADR-015
(dependency compile and replay). The proof/replay contracts (ADR-013 O-24 to
O-27, ADR-011 E9) are treated as accepted. Gate QSL-20's claimed-module
Kani discharge check changes none of them, and this
record changes none of their wire members.

**Protocol and frame, implemented.** #218 landed (FR-105, FR-112 to
FR-116):

- I1 reads each operation's `frame` from the domain package into
  `OperationEffect { modifies, creates, deletes: Vec<DeclarationKey> }`
  (`qsl-semantics/src/model/intake.rs:1410-1440`, `:2411-2480`;
  `model/domain_package.rs:244-253`). No QSL source production spells a
  frame.
- S3 holds `CheckedOperationFrame { operation, anchor: NodeKey, frame:
  NodeKey, frame_origin: Origin }`, one per (declaring type, operation name),
  for each operation a clause or attempt names (`check/state_clause.rs:62-68`,
  `check/mod.rs:489-508`), and `CheckedAttempt { declaration, anchor, frame,
  contracts: Vec<NodeKey> }` (`check/protocol_clause.rs:91-103`). A scoped
  anchor resolves to a `ProtocolNodeId` inside S3 only
  (`check/protocol_clause.rs:532-608`).
- S4 emits `state`/`operation_anchor`, `state`/`frame` (the QSpec FR-340
  term, each `modifies` entry a `FrameField { declaration: NodeRef, name }`)
  and `state`/`state_clause` (`check/lowering/state.rs:198-428`). No
  protocol, scoped anchor or attempt node is emitted.
- S6a resolves the frame by `NodeKey` and compares the admitted pre and post
  observations against `OperationEffect` by `DeclarationKey`
  (`qsl-eval/src/value/expression/s6a/protocol_clause.rs:141-163`;
  `model/population.rs:1440-1546`).
- Replay: `WitnessEnvelope<FrameCounterexample>` with payload `{ operation:
  FrameOperation { object: QualifiedName, operation: Identifier }, anchor,
  frame: WireNodeId, occurrence: OccurrenceKey, invocation: DocumentRef,
  change: ClaimedChange }` (`qsl-replay/src/witness/frame.rs:133-152`).
  `replay_frame` recompiles, checks `package_id`, resolves the operation, then
  compares the frame, occurrence and anchor ids before admission
  (`qsl-replay/src/execute/frame.rs:167-395`).
- Requirement records: one `operation-contract` record per clause and per
  frame, keyed by the frame's occurrence (`check/mod.rs:1452-1572`).
- Tests: TC-462, TC-463, TC-514 and TC-515 pass locally; TC-463 step 1 (the
  I2 read-back) passes through IR-370's `reaches_field` reference-edge check.
  `spec/tests.md` still lists TC-510
  to TC-513 as planned although traced tests for them exist; this record
  cites them only as specified.
- SEAM-3 handoffs to IR are deleted (M-6d). `src/protocol_artifact`
  remains for the root crate's native, state and temporal paths only; no
  `qsl-*` crate imports it.

**Refinement, not implemented.** #191 (QSL-40) and #192 (QSL-39) specify
corpus-differential `xtask` gates with no solver, no family hook and no
routing (ADR-011 "Answers to ADR-012 §13.1"; ADR-012 §11). QSpec#116 (STD-9)
records them as "corpus-differential test gates in QSL" for refinement uses
1 (spec versioning) and 2 (profile layering), and mints V1-TOOL-011 ("a
superseding specification edition admits every result its prior edition
admitted", → #191) and V1-TOOL-012 ("every parent profile's refusal [is] its
child profile's refusal under profile layering", → #192). QSL-40 (data):
"for every versioned spec pair in the corpus, v2 admits everything v1
admitted". The profile hierarchy is QSpec AD-003 "Accepted profile
hierarchy"; QSpec FR-250 is temporal profile identity. QSpec FR-290 and
FR-057 list "Refinement between two operation contracts or state models (a
relation-family refinement gate)" as `operation-contract`, one claim per
clause implication, and ADR-012 §3 says the gates "emit
`operation-contract` claims". QSpec FR-177 is protocol implementation
refinement (FR-290 `refinement`). Spine `compile` resolves a header profile
by its identity: a header naming any QSpec AD-003 layer resolves to that
layer (FR-110, "Layer selection"), and the catalog's qualification rows are
selected by QSpec's package-selection rules, not by the author. No
`Relation` checker, form or gate exists: only `FamilyKind::Relation`
(`qsl-semantics/src/family/mod.rs:101`) and a CST `RelationClause`
production that S2 refuses (`qsl-forms/src/value.rs:1044`).

**Abstraction relation, not implemented.** QSpec FR-353 (merged) fixes the
relation: authored, never inferred; a model object maps to one
implementation state representation, a population to one collection
representation, an operation frame to one implementation function's
parameters, result and framed state; keys are the QSpec AD-006
effective-declaration key (domain package identity, IR node identity,
`sha256-jcs`) and, for a frame, its QSpec FR-013 frame identity and QSpec
FR-012 pre/post/result anchor identities; duplicate or conflicting bindings
refuse `invalid_model_binding`/`conflicting-binding`; emission referencing an
unbound element refuses before generation with `missing_declaration`/
`missing-name`, scoped to that item; rebinding is a new relation revision.
QSpec FR-290 gives it no capability kind ("a premise of the
`operation-contract` claim it binds"). QSpec has no surface syntax and no
`quire.checked-package/v2` member for it.

**Downstream owners.** The QSL-16 body's links for "#84" and "#136" point at
unrelated QSL items (QSL #84 is IT-010; #136 is an intake PR). The owners are
agent-ix/quire-contract-codegen#84 (Linear IR-32, V1-E10, Verus emission
"through the abstraction relation exported by QSL V1-A15", Backlog) and
agent-ix/quire-contract-ir#136 (Linear IR-33, V1-E09, SMT/runtime parity
corpus, Backlog; IR-33 blocks IR-32). Frame lowering is
agent-ix/quire-contract-ir#109 (IR-89; its lowering half is now IR-339,
Backlog). Kani contracts and frame harnesses are
agent-ix/quire-contract-codegen#49 (IR-93, Backlog).

## Decision

### 1. Protocol and frame

#### PF-1 Identities

Every protocol/frame identity has one ADR-013 owner. The landed code
conforms, except for the defects G-1 to G-3 below.

| Concept | S2 | S3 (owner: `check`) | S4 wire | Replay | ADR-013 row | Equality |
| --- | --- | --- | --- | --- | --- | --- |
| Clause | `StateClauseForm` (spelled, no identity) | `CheckedStateClause.identity: NodeKey` of the `state`/`state_clause` node; the name is outside the key | `node_id` in `quire.checked-semantic-node/v1` | `WireNodeId`, a `NodeKey` only by lookup in the recompiled package | O-09 | normalized |
| Frame | none (no source production) | `CheckedOperationFrame.frame: NodeKey`; subject `OperationEffect` | `state`/`frame` node | `FrameCounterexample.frame: WireNodeId` | O-08 (amended, PF-5) | normalized; subjects compare as sets of `DeclarationKey` and (node, name) pairs |
| Operation anchor | none | `CheckedOperationFrame.anchor: NodeKey`, one per (declaring type, operation name) | `state`/`operation_anchor` node | `FrameCounterexample.anchor: WireNodeId` | O-09 (FR-105) | normalized |
| Scoped anchor | `ScopedAnchorForm { site, anchor, scope, channel }` | `ProtocolNodeId`, S3-internal | none (ADR-012 §12.2) | none | none: it never leaves S3 | index within one protocol form |
| Occurrence | span | `OccurrenceKey { node: WireNodeId, origin }`; the frame's `generated` occurrence keys its record | `source_map` entry | `FrameCounterexample.occurrence` | O-07 | lexical over (node id, role, ordinal) |
| Model element | `NameForm` | `DeclarationKey { package, node }` via `ModelCorrespondence` (`NodeKey → DeclarationKey`); object types also carry `EffectiveId` | model-owned node ids | `DeclarationKey` in `ClaimedChange` | O-03, O-04, O-05 | declared |
| Member | spelled field name | (declaring `object_type` node, member name): `FrameField` | `{kind: "field", declaration, name}` | field name within `ClaimedChange::FieldWrite` of a resolved object | O-06 | declared |

A frame is read and written only by these identities. The frame comparison
at S6a and in replay binds no display name.

#### PF-2 Checked input and the proof/replay path

- **Package authority.** The frame's authority is the domain package admitted
  at I1 (ADR-013 O-01, FR-103). A run or replay request carries no frame and
  no permission (FR-115, QSpec FR-013-AC-3). The in-process `CheckedPackage`
  is the authority for S6a, run and replay; the v2 bytes are the authority
  for IR (ADR-011 E5). No stage builds checked typestate from v2 bytes
  (ADR-013 R-10).
- **Proof path.** A frame is an `operation-contract` requirement record
  (FR-057, FR-104) routed by `qsl-route` and settled by CG `negotiate_*`
  (ADR-012 §7.2). Until IR-339 and agent-ix/quire-contract-codegen#49 land,
  IR and CG hold explicit `unsupported` arms, so a frame record settles
  `unsupported` (ADR-012 §12.2). The two land independently; each removes
  its own arm.
- **Replay path.** A frame counterexample replays through the layer-6
  `replay` facade and the `ProtocolClause` S6a arm (FR-116), the ADR-011 E9
  path, with the ADR-013 O-25 to O-27 carriers. The decode from IR's witness
  bindings waits on the same two tickets; the in-process envelope is the
  input until then.

#### PF-3 Entry selection is the one post-check name lookup

ADR-013 R-06 and ADR-011 §1 name one post-check name lookup, the replay
`QualifiedName` at E9. Four layer-6 entries select the item under test by an
authored name: FR-100 (function), FR-109 (state clause, Boolean function),
FR-115 (operation frame) and FR-116 (frame replay). They are one rule:

- A layer-6 entry selects by an authored name: a function or a state clause
  by one identifier segment (FR-100's `function` rule, FR-109's
  `Function { name }` and `ClauseSelection.name`, whose `select` step refuses
  any other shape), replay's function by a typed `QualifiedName` (FR-098),
  and a frame by an `OperationName` of three identifiers (model alias, object
  type, operation; TK-1). It resolves the name once, against the package it
  has just compiled, to a `NodeKey`. Everything after selection is keyed by
  `NodeKey`.
- If the name resolves to no item of the selected kind, the entry reports
  stage `select`, `missing_declaration`/`missing-name` (FR-109, FR-115), and
  replay refuses `missing_declaration`/`missing-name` (FR-116).
- At E9, after the `package_id` check (FR-098), replay compares each resolved
  identity with the packet's: for a frame, the frame node and its occurrence
  and the anchor node. A mismatch refuses `stale_dependency`/
  `revision-mismatch` naming both, before admission (FR-116). This detects a
  packet whose members disagree with a recompile of the same `package_id`; a
  stale packet already refuses at the `package_id` check.
- A member name inside an O-06 pair, and an operation `Identifier` looked up
  under a resolved `DeclarationKey`, are declared keys, not names under R-06.
  No other post-check code resolves a name.

**G-1.** `CheckedGraph::operation_frame(context: &str, operation: &str)`
(`qsl-semantics/src/check/mod.rs:1807-1831`) matches `object.name()` against
the string `format!("{}::{}", model, object)`, built by its one production
caller `qsl_replay::spine::clause::resolve_frame`
(`qsl-replay/src/spine/clause.rs:883-894`). `resolve_frame` is reached from
the FR-115 run (`spine/clause.rs:867`) and the FR-116 replay
(`execute/frame.rs:187`, after `operation_name` at `:348-357` converts the
payload's `FrameOperation`). It selects by a formatted display string where
PF-3 requires a typed name. TK-1 fixes it (§6).

#### PF-4 The frame packet's identity carriers

`WitnessEnvelope` carries `selected_function: QualifiedName`,
`occurrence_key` and `clause_node` (`qsl-replay/src/witness.rs:343-359`);
`FrameCounterexample` carries its own `operation`, `occurrence` and `frame`.
`replay_frame` reads the payload's members and not the envelope's three.

**G-2.** Two carriers of one identity, one unread, can disagree silently.
Decision, with no wire change:

- For a frame packet, the envelope's `clause_node` is the frame node, its
  `occurrence_key` is the frame's `generated` occurrence, and its
  `obligation_identity` is the O-09 obligation identity of the frame's
  `operation-contract` record (the frame node and that occurrence in place
  of a clause's).
- If the envelope's `clause_node` differs from the payload's `frame`, or its
  `occurrence_key` from the payload's `occurrence`, `replay_frame` refuses
  `stale_dependency`/`revision-mismatch` naming both, before it recompiles.
- `replay_frame` reads no `selected_function`: the envelope constructor
  still requires one, and its value is not interpreted for a frame packet.
  A selection member that names a function or a frame operation is a change
  to QSpec FR-323's `selection` and CG's packet members, so it is Q-4, not
  QSL work.

TK-2 implements the first two bullets (§6).

#### PF-5 Frame representations

Two frame representations are live, both built from one `OperationEffect`
in one compile (`check/lowering/state.rs:324`): `OperationEffect` (what S6a
compares) and the v2 `SemanticTerm::Frame` (what S4 emits and keys). ADR-013
O-08's public type becomes: `CheckedOperationFrame` at S3, the
`state`/`frame` node at S4, and its subject `OperationEffect`.

**G-3.** `check::identity::{Frame, FrameSubjects, ResolvedFrameSubjects}`
(`check/identity.rs:280-348`, re-exported at `check/mod.rs:191-192`) is a
third representation that only its own test uses
(`frame_subjects_resolve_only_through_the_recorded_correspondence`, TC-248,
FR-088-AC-2). TK-3 fixes it (§6).

#### PF-6 Versions

| Input | Version carrier | Check | Effect of a change |
| --- | --- | --- | --- |
| Domain package (frames) | `DomainPackageRef` identity, version, `sha256-jcs` digest (O-01) | I1 digest recompute; document `model` header equality (`invalid_model_binding`/`wrong-model-selection`) | A new version changes the model-owned node ids (O-04) and so the `package_id`; an old packet refuses by FR-098's `package_id` rule |
| Checked package | `quire.checked-package/v2`, `package_id` (O-02, O-22) | `WitnessEnvelope::reconstruct` pins the contract; `replay_frame` checks `package_id` | A meaning edit changes `package_id`; replay refuses by FR-098 |
| Snapshots and invocation | FR-001 four labels plus `sha256-jcs` digest (FR-106) | label and byte checks at admission | `stale_dependency`/`revision-mismatch` or `byte-digest-mismatch` |
| Frame payload | none | identity equality only | Staleness is detected through content-addressed identities (O-02, O-04), never through a version field |

#### PF-7 No protocol-internal redesign

No redesign is authorized. Each change scenario below lands inside the
modules ADR-012 §12.2 and §15.3 name:

1. **An S2-parsed construct gains its check** (for example `compensate`,
   parsed as `ProtocolNodeKind::CompensateTemplate`, `qsl-forms/src/syntax.rs:1225`,
   and refused by S3's `covered_kind`, `check/protocol_clause.rs:389-405`):
   classify it in `covered_kind`, check it in `check::protocol_clause`, lower
   it in the `ProtocolClause` lowering.
2. **A temporal atom over a protocol operation**: `TemporalTrace` reads the
   checked operation and clause types in the `check` core (ADR-012 §1); no
   `ProtocolClause` change.
3. **Kani frame harnesses** (IR-339, agent-ix/quire-contract-codegen#49): IR
   and CG arms only.
4. **An abstraction relation over frames** (§3): reads operation member
   identities; no `ProtocolClause` change.
5. **The protocol transition system** (amended by ADR-027): `ProtocolSystem`
   lands in layer 5 `simulation`, reads the checked protocol clause S3
   produces and leaves the `ProtocolClause` family's identities unchanged.

The coupling found (G-1 to G-3) consists of local defects with bounded
tickets. SEAM-3 and the composed protocol checker are not on the spine path,
and their deletion is owned (PF-8).

The `unsupported_construct`/`not-yet-implemented` refusal that S3 uses for an
unimplemented protocol construct names a cause that QSpec's catalog revision
`1-draft.8` does not list for `unsupported_construct`
(`declaration-form`, `expression-form`). Q-6 asks QSpec for it.

#### PF-8 Lane deletions (ADR-011 §7.3, T-3)

This record implements nothing, so it deletes nothing. The ticket's lane
exit criteria map to the PRs that own each deletion:

| Lane item | State | Owner |
| --- | --- | --- |
| M-6d SEAM-3 handoffs to IR | deleted | Done |
| M-6d composed emission B8, B9 and the composed `ProtocolClause` checker (`src/linking/composed/scopes/protocol*`) | present, not on the spine | QSL-316 (M-6e) |
| SEAM-3 reads feeding `state` and `temporal`; `protocol_artifact` native and writer | present, root crate only | M-6c (#188, #189) |
| M-6e `Relation` composed code | none exists | none: #191, #192 and #198 delete nothing |

### 2. Refinement comparison (#191, #192)

#### RF-1 What the gates are

The #191 and #192 gates are test gates: `xtask refinement` subcommands that
run a corpus through the spine and compare structured outcomes. They record
no requirement record, emit no witness envelope, and have
no `Relation` family hook (ADR-011, ADR-012 §11). A passing gate is evidence
over its corpus only. It never settles `proved` for any claim. The only
backend request either gate makes is #192's negotiation of each layer's
witness item (RF-3).

A spec-versioning case tests one clause implication of the QSpec FR-290 row
"Refinement between two operation contracts or state models" on one input.
That row names the claim a proof route would request, one
`operation-contract` claim per implication; no V1 ticket owns that route.
This amends ADR-012 §3's `Relation` row and FR-057's claim-form row, and Q-3
asks QSpec to amend its FR-290 row and to re-trace V1-TOOL-011 and V1-TOOL-012
away from QSpec FR-177 and TC-208.

#### RF-2 Spec versioning (#191, V1-TOOL-011)

**Pair.** A pair is two complete-V1 units: the prior and the superseding
revision of one specification. Both have the same FR-001 authority and
identity. The corpus states each pair's direction explicitly, by which unit
it names `prior` and which `superseding`; the gate reads no order from, and
compares nothing about, the revision labels. Both revisions compile in the
running build, against the same domain packages and libraries. No recorded
outcome is needed.

**Corpus** (amended 2026-10-01, FR-340). A corpus is one directory. Each
pair is one subdirectory holding a `pair.json` file and the files it names.
`pair.json` names, by path relative to its own directory, every byte the
pair's runs read: the `prior` and `superseding` sources, each with its four
FR-001 labels; the pair's domain packages (FR-056 input) and dependency
input (FR-099); and its `cases`, each a selection, the snapshot and
invocation files it reads, and any of the four limit sets it states. The
file is the gate's input, not a record of anything: it holds
no expected outcome. A pair file that cannot be read, holds an unknown or
missing member, names a path outside its directory, or names two sources
whose authority or identity differ is one tool-failure result naming the
file and the defect; the other pairs still run.

**Case.** A case is one FR-109 `ClauseRunRequest` selection and its input
(`Clause` with its snapshot or invocation, `Function` with its arguments and
snapshot, or `Frame` with its invocation), and the four limit sets the
request takes (`limits`, `observation_limits`, `model_limits`,
`accounting`; a case that states none uses their published defaults). It is
run through `qsl_replay::spine::run_clause` against each revision of its
pair, with no expected `package_id`. A case is named in every report by the
pair's two `RawSourceRef`s (O-12) and the selection. It is never named by a
display string.

**Compile classification.** One classification of a spine `compile` result,
shared by #191 and #192. It is an exhaustive `match` over `CompileRefusal`
with no wildcard arm, reading every cause of the `Assembly` and `Check`
vectors through an all-causes accessor that #191 adds (not `code()`, which
reads the first). The first rule that matches wins:

1. `Ok` → admitted.
2. any cause with code `runtime_invariant`; `Intake` other than its limit
   cause, `Profile`, `DependencyInput`, `Import` other than its depth limit,
   `Dependency`, `Link` or `Emit` → tool failure (the case's setup is
   broken).
3. `Source`, `Forms`, `Assembly` or `Check` holding a typed refusal cause (a
   code other than `stage_limit_exceeded`, `unsupported_construct` and
   `unsupported_projection`) → refused.
4. any `stage_limit_exceeded` cause (`FormsFailure::Limit`,
   `UnitIntakeCause::Limit`, `AssemblyCause::TypeLimit`,
   `ImportRefusal::DepthLimit`, or an `Assembly` or `Check` cause with that
   code) → incomplete.
5. `Omitted`; any `unsupported_projection` cause; any `unsupported_construct`
   cause whose cause is `not-yet-implemented` → unsupported (a QSL
   implementation gap).
6. otherwise every cause is `unsupported_construct` with a catalog cause
   (`declaration-form`, `expression-form`) → prohibited (the selected profile
   prohibits the form, QSpec AD-003).

**Run classification.** The gate classifies each run once, by an exhaustive
`match` over `ClauseDisposition` with no wildcard arm, never by message text
(ADR-011 FB-02):

| `ClauseDisposition` | Class |
| --- | --- |
| `Compile(r)` | the compile classification of `r` (admitted cannot occur) |
| `UnknownLanguage`, `StalePackage` | tool failure (the gate supplies neither an extracted source nor an expected `package_id`) |
| `MissingName` | absent |
| `NotAPredicate` | refused |
| `Admit(Refused)`, `ArgumentRefusal` | refused |
| `Admit(Incomplete)` | incomplete |
| `Admit(Fault)`, `EvaluateFault` | tool failure |
| `Evaluate(Completed(Boolean(true)))` | admitted |
| `Evaluate(Completed(Boolean(false)))`, `Evaluate(Refused)`, `Evaluate(Undefined)`, `FrameViolation` | refused |
| `Evaluate(Completed(Integer))` | tool failure (FR-109 selects only `Boolean` functions, so it cannot occur) |
| `Evaluate(Incomplete)` | incomplete |

**Comparison.** Per case, `P` is the prior revision's class and `S` the
superseding revision's. Rows are matched in order; the first match wins.

| `P` | `S` | Case result |
| --- | --- | --- |
| tool failure | any | tool failure |
| any | tool failure | tool failure |
| incomplete, at any stage (amended 2026-10-01) | any | unresolved (incomplete) |
| unsupported, at any stage (amended 2026-10-01, QSpec FR-452) | any | unresolved (unsupported) |
| any other class, at stage `compile` or `select` | any | tool failure (the prior revision must compile and select; the case is malformed) |
| admitted | admitted | holds |
| admitted | absent, for a `Clause` or `Frame` selection | holds: the constraint was dropped (a frame exists only while a clause or attempt names its operation, FR-105) |
| admitted | refused, prohibited, or absent for a `Function` selection | regression, naming the case, both classes and the superseding disposition's codes |
| admitted | unsupported | unresolved (unsupported) |
| admitted | incomplete | unresolved (incomplete) |
| not admitted | any | not applicable |

A typed compile refusal of the superseding revision is class refused for
every case of the pair, so every case the prior revision admits is a
regression.

#### RF-3 Profile layering (#192, V1-TOOL-012)

Q-5 is answered (amended 2026-10-01; QSpec FR-453): every QSpec AD-003 layer
is header-selectable, and the gate compares all five `requires` edges (state
core to state queries, state queries to state graph, state core to complete
value, state graph to complete model, complete value to complete model).
FR-110's layer selection resolves a header naming any of the five layers, and
S3 admits each declaration under that layer's admitted-form set, so both
sides of every edge compile in the running build. A hand-written "parent
refused" expectation that no compile checks is not V1-TOOL-012 evidence, and
this record does not adopt one.

A #192 case is one edge and two units, byte-equal except for the identity
string of one header `profile` declaration, each compiled through spine
`compile` and classified by RF-2's compile classification. The gate also
checks that each parent layer is a subset a user would select: each layer
admits its witness unit, and the witness item, requested through QSpec
FR-331 negotiation with the layer's `witness_request` FR-290 kind and
extent once from each candidate backend by name, settles `supported` under
at least one candidate (a failure lists each candidate's disposition); and each parent prohibits its child's
distinguishing form (QSpec FR-453-AC-5, AC-6; FR-345).

Per case (parent `R`, child `C`), first match wins: tool failure on either
side → tool failure; `R` refused and `C` refused → holds, both codes
reported; `R` refused and `C` admitted or prohibited → regression, naming
the case, the edge and `R`'s codes; `R` refused and `C` unsupported or
incomplete → unresolved; `R` incomplete → unresolved; otherwise not
applicable. A prohibited form is not a refusal a child preserves: a child
profile admits forms its parent prohibits (QSpec AD-003).

#### RF-4 Gate verdict and exit

The gate's report lists every case with its result, ordered by the case's
`RawSourceRef`s and selection, and two runs over one corpus give byte-equal
reports. Its verdict is the highest-severity case result under QSpec
FR-301's order, and its exit code is QSpec FR-301's code for it:

| Case results present | Verdict | QSpec FR-301 exit |
| --- | --- | --- |
| any tool failure | tool failure | 30 |
| any unresolved (unsupported) | unsupported | 21 |
| any unresolved (incomplete) | incomplete | 22 |
| any regression | violation | 10 |
| otherwise | success | 0 |

A case whose class on either side is incomplete names, in the report, the
limit it reached, that limit's value and the case member that raises it
(amended 2026-10-01, FR-344).

Every regression is listed whatever the verdict, so an unsupported case never
hides a regression from the report. An unresolved case is never promoted to
holds. This extends ADR-012 §13.5's `Relation` row (pass → success, mismatch
→ violation) with unsupported, incomplete and tool failure.

#### RF-5 Seeded-regression oracle

The seeds and controls live in a test-only corpus that the gate's real run
does not read (the real corpus is under `tests/fixtures/refinement/`).

1. **Seeded regression (#191).** One real pair whose superseding revision
   tightens a ConfigVersion clause, and one case the prior revision admits
   and the superseding refuses. Expected: violation, exit 10, naming exactly
   that case. The rest of the test corpus holds.
2. **Oracle independence.** Each expected verdict and case result is a
   literal in the test. No test computes its expectation with the gate's
   classification or comparison code (ADR-011 Decision item 8, §2.3, applied
   to a test gate).
3. **Mutation controls.** Each turns a test red: comparison returns holds for
   every case (caught by the seed); it returns regression for every case
   (caught by the holding cases); `unsupported` classified as admitted
   (caught by a control whose superseding compile refuses
   `unsupported_construct`/`not-yet-implemented`); incomplete classified as
   holds (caught by a control whose superseding run has a one-unit
   `accounting` work limit). The comparison is a pure function of the two
   classes and the selection kind; a unit test over every class pair of the
   comparison table catches a tool failure hidden by "not applicable" and
   any row reordering, with no fault injected into a compile.
4. **Determinism.** Two runs give byte-equal reports.

#192's seed and controls follow the same rules, one seed per edge.

#### RF-6 Version effects

- A new revision of a specification adds a pair; old pairs keep running.
- A new edition or `root` revision changes no pair's meaning: both revisions
  of every pair compile under the running build, so the gate compares within
  one build and never across two. A header profile carries only its
  identity (FR-110), so a new revision changes no source: each build
  compiles every unit under its own definition of that identity.
- The gate compares classes, never `package_id`s.

### 3. Abstraction relation (#198)

#### AR-1 Where it is authored

The relation is a `Relation` family declaration in QSL source, parsed at S2,
checked at S3 and carried in the checked package (ADR-012 §3). It is not a
domain-package declaration: FCD's semantic IR describes the model, not its
implementation. Its surface spelling is QSpec FR-450's `abstraction-decl`
(Q-1, decided).

#### AR-2 Keys and binding values

| QSpec FR-353 element | QSL key | Binding value |
| --- | --- | --- |
| Model object | the object type's `DeclarationKey { package, node }` (O-03) | `ObjectBinding { rust_type: RustPath, fields: map field Identifier → RustField }` |
| Population | the population declaration's `DeclarationKey` (the `population_key` FR-089 mints `PopulationId` from) (O-03) | `PopulationBinding { collection: RustPath }`: the path from the implementation state root to the collection |
| Operation frame | `OperationKey { declaring: DeclarationKey, operation: Identifier }` (O-03 plus the operation identifier) | `FrameBinding { function: RustPath, receiver: RustReceiver, parameters: map operation parameter Identifier → Rust parameter identifier }`; a `RustReceiver` is `self` or one Rust identifier naming a function parameter (QSpec FR-450) |

- **Anchors.** An operation has one anchor and one frame (FR-105). Its QSpec
  FR-012 pre, post and result anchors are the implementation function's
  entry, exit and return value, so they are not key members: the parameters
  are read at entry through `parameters`, the result is the function's
  return value, and the framed state is read at entry and exit through
  `receiver`. A framed field is read through its object type's
  `ObjectBinding.fields`, and a created or deleted object through its
  population's `PopulationBinding`, so each Rust representation has one
  carrier.
- **Frame identity.** A frame's QSpec FR-013 frame identity and its anchor
  identities are functions of `OperationKey`. A binding therefore exists for
  an operation that no clause names, and S3 relates it to the
  `state`/`frame` and `state`/`operation_anchor` nodes when a clause or
  attempt names the operation. QSpec FR-353-AC-1 accepts this derived key
  (Q-7, decided).
- An inherited operation anchors at its declaring type (FR-105), so it has
  one frame binding, at the declaring type. A subtype whose implementation
  function differs cannot be bound separately: QSpec FR-353 maps one frame to
  one function.
- QSpec FR-353's "AD-006 effective-declaration key" is QSL's O-03
  `DeclarationKey`. It is not O-05's `EffectiveId`
  (`quire.model.effective-declaration/v1`), which keys normalized
  declarations.

**Implementation side.** `RustPath` is a non-empty sequence of Rust
identifiers, and `RustField` is a Rust identifier or a tuple-field index. An
identifier is `IDENTIFIER` of the Rust Reference for edition 2021: a
non-keyword identifier or a raw identifier (`r#type`). A tuple-field index is
a decimal integer with no leading zero. These are lexical keys (ADR-013 R-06
contract-defined keys) and compare by bytes. QSL checks their syntax only and
never resolves them against Rust code. CG's generator resolves them (§6).

Amended by ADR-025 MX-2 to MX-4. For a subject with a weak `parallel`, a
proof transfers to the code bound here under preconditions on these bindings:
each shared location with an atomic access binds to a Rust atomic, and one
with only non-atomic accesses to plain state; each attempted operation's
function performs one access of its classified kind with at least the
attempt's declared ordering; and the subject's race-freedom item is proved
when it has non-atomic accesses. QSL checks the binding syntax only, as
above, and states these as preconditions.

#### AR-3 Checked form and refusals

S3 builds `CheckedAbstractionRelation`, in the `check` core, holding at most
one binding per key. It records no requirement record (FR-057: no kind) and
has no S6a arm (ADR-012 §2). S3 refuses, naming the key:

- a model key that resolves to no admitted declaration of the selected
  domain package, or an `OperationKey` whose type declares no such
  operation: `missing_declaration`/`missing-name`, naming the key and its
  owning `DomainPackageRef` identity;
- two bindings for one key, equal or not: `invalid_model_binding`/
  `conflicting-binding`, naming both bindings and the key (QSpec
  FR-353-AC-4); two bindings are equal when their values are equal member by
  member, lexically;
- a `FrameBinding` whose `parameters` map names a parameter the operation
  does not declare, omits one it declares, or maps two parameters to one Rust
  parameter; or an `ObjectBinding` whose `fields` map names a field the type
  does not declare: `invalid_model_binding`/`malformed-declaration`, naming
  the entry;
- a malformed `RustPath`, `RustField` or `RustReceiver` segment: `invalid_model_binding`/
  `malformed-declaration`, naming the segment and its span.

S3 does not refuse a relation that leaves elements unbound. QSpec FR-353
scopes the unbound refusal to the item that references the element (AR-4).
There is no totality check.

#### AR-4 Export and the unbound refusal

- **One authority.** The relation is part of the checked package, as a v2
  node whose tag, form and body QSpec FR-451 spells (Q-2, decided). Kani and Verus read it
  only from the v2 bytes (ADR-011 FB-05). There is no side file.
- **Implementation export.** A layer-4 `package` function takes the checked
  package and the requested items, each a requirement record's occurrence key
  (ADR-012 §13.5). It returns, per item, either the item with the bindings it
  references or a refusal. Its caller is the orchestrating driver (ADR-011
  T-13), which calls it after S4 and before `route` and E7, as it calls
  `route`. It names no `BackendId` (ADR-012 §11). For an emission onto
  implementation code, the driver puts only the bound items in the QSpec
  FR-331 request it sends CG, so CG receives no new input; CG reads the
  bindings from the v2 node.
- **Referenced elements.** An item references: the receiver's static object
  type of every model member in its checked claim (the O-06 declaring node,
  which is the static type even when a supertype declares the member,
  resolved to its `DeclarationKey`); the population of every extent domain
  (ADR-012 §15.7); and, when the record's node is a frame, precondition or
  postcondition, the `OperationKey` of its operation.
- **Refusals**, per item:
  - A requested occurrence key that names no requirement record refuses
    `missing_declaration`/`missing-name`, naming the key.
  - An item that references an element with no binding refuses
    `missing_declaration`/`missing-name`. The refusal names every unbound
    element of the item, in ascending key order, each with its owning
    `DomainPackageRef` identity (QSpec FR-353-AC-3).
  - A refused item returns no bindings; a bound item returns every binding it
    references. Other items continue.
- CG's own unbound check (CG#84: "A claim whose model elements have no bound
  relation refuses") guards requests formed outside the driver, as QSpec
  FR-290's per-item `invalid-request` does (ADR-012 §7.2).
- **Relation revision.** The relation is a v2 node (QSpec FR-451), so its
  node ids enter the `package_id` (O-02), so rebinding changes the `package_id`. Every
  generated contract carries the `package_id` it was generated from (O-25),
  so it is never reinterpreted (QSpec FR-353-AC-5). There is no separate
  revision field.

#### AR-5 Capability dispatch and proof/replay

- The relation requests no backend (QSpec FR-290, FR-057). The claim it binds
  keeps its own kind (`operation-contract` or `value-validity`) and extent
  (ADR-014 §4) and is routed and negotiated as ADR-012 §7.2 states.
- A Verus candidate is a backend descriptor (its QSpec FR-331 manifest) with a
  CG `negotiate_*` arm (ADR-012 §12.3). Its advertised mode is CG's to state
  (QSpec AD-010); QSL adds no kind and no mode.
- A counterexample from a claim that used the relation replays as clause
  expressions through S6a (ADR-012 §8). The relation is not an S6a input and
  has no witness payload.

#### AR-6 What downstream receives

| Consumer | Versioned inputs | Capability | QSL-side contract |
| --- | --- | --- | --- |
| CG#84 Verus (IR-32) | `quire.checked-package/v2` with the Q-2 relation node; `package_id`; the QSpec FR-331 request, holding only bound items; the Verus QSpec FR-331 manifest and tool pin | the bound claim's QSpec FR-290 kind; Verus's mode per its manifest | AR-2 keys and values; unbound items never reach CG |
| CG Kani (QSpec FR-196, CG#49) | as for Verus, with Kani's bounded mode (ADR-014 §6) | as the claim | as for Verus |
| IR#136 SMT/runtime parity (IR-33) | `quire.checked-package/v2`; the QSpec FR-323 runtime request | the claim's kind | none: parity compares model-level encodings with `runtime::execute` and reads no abstraction relation |

### 4. Mapping summary

| Mapping | Types and owners | Conversions | Version effect | Failure oracle | Tests | Tickets |
| --- | --- | --- | --- | --- | --- | --- |
| Protocol/frame | PF-1; `check`, `package`, `qsl-replay` | ADR-013 C-02, C-03, C-11, C-13, C-14 | PF-6 | stale ids refuse `revision-mismatch` (PF-3, PF-4); `frame_violation`/`unauthorized-change` | TC-462, TC-463, TC-514, TC-515 pass; TC-510 to TC-513 specified; each TK adds its own (§6) | TK-1 to TK-4 |
| Refinement | `xtask refinement`; spine `run_clause` and `compile`; `ClauseDisposition`, `CompileRefusal` | disposition → class and compile result → class (RF-2) | RF-6 | RF-2 and RF-3 tables; RF-5 seed and controls | RF-5 | #191; #192 |
| Abstraction relation | AR-2, AR-3; `check` core; layer-4 export | checked relation → v2 node (Q-2); export → driver → CG | AR-4 relation revision | AR-3 and AR-4 refusals | AR-7 | #198 slices; Q-1, Q-2 |

#### AR-7 Abstraction-relation tests

QSpec TC-268 verifies QSpec FR-353-AC-1 to AC-5. QSL's share:

| QSpec FR-353 AC | QSL test (owner) |
| --- | --- |
| AC-1 | a ConfigVersion object, its population and `attemptUpdate`'s frame bind, one binding each (#198) |
| AC-2 | Kani and Verus read the relation through QSpec AD-010 negotiation (CG#49, CG#84; not QSL) |
| AC-3 | an item referencing an unbound element refuses naming every unbound element, while a sibling item with bound elements continues; an unknown occurrence key refuses (#198) |
| AC-4 | a duplicate and a conflicting binding each refuse `conflicting-binding` naming both; a `parameters` map that omits a parameter and a malformed `RustPath` segment each refuse `malformed-declaration` (#198) |
| AC-5 | rebinding one field changes the `package_id` (#198 feature slice) |

### 5. Named interfaces for the dependent tickets

**#198 (QSL-36)**, in two slices:

- **Enablement**, which waits on nothing: `CheckedAbstractionRelation`,
  `OperationKey`, the binding value types, `RustPath` and
  `RustField` and `RustReceiver`, the AR-3 refusals and the AR-4 export,
  tested on relations built in the test from checked packages.
- **Feature**, on QSpec FR-450 and FR-451: the S2 form, the S4 relation node,
  AC-5, #198's spine exit (one ConfigVersion model element binds to a Rust
  struct and exports through the checked package; an unbound element
  referenced by an emission request refuses by name; model admission
  unchanged, shown by TC-462 and TC-514 passing unchanged) and ARCH-G4
  scenario 8.

**#191 (QSL-40)** builds `xtask refinement versioning`: the pair and case
reader, the all-causes accessor on `CompileRefusal`, RF-2's classifications
and comparison, the RF-4 report and exit, the corpus under
`tests/fixtures/refinement/`, and RF-5's tests.

**#192 (QSL-39)** builds `xtask refinement layering` over all five AD-003
edges, on RF-2's compile classification, RF-3, RF-4 and FR-110's layer
selection. It cites QSpec AD-003 and
V1-TOOL-012, not QSpec FR-250.

**Downstream asks.**

| Owner | Ask |
| --- | --- |
| CG#84 (IR-32), CG#49 (IR-93) | read the relation from the QSpec FR-451 v2 node; resolve each `RustPath`, `RustField` and `RustReceiver` and refuse one that names no item, naming the path (CG chooses the catalog cause); keep the unbound guard |
| Orchestrating driver (ADR-011 T-13, QSL #248) | call the AR-4 export before `route`, and put only its bound items in the QSpec FR-331 request for an emission onto implementation code |
| IR#136 (IR-33) | nothing from this record (AR-6) |
| IR-339, CG#49 | replace their `unsupported` frame arms (PF-2) |

### 6. Tickets

These are proposed; the team lead files them.

| ID | Scope | Verification | Repository |
| --- | --- | --- | --- |
| TK-1 | G-1: retype `qsl_replay::spine::OperationName` to (model alias `Identifier`, object `Identifier`, operation `Identifier`); add one resolver in `check` from it to (object `DeclarationKey`, operation `Identifier`), the `DeclarationKey` carrying its domain package, refusing `missing_declaration`/`missing-name` when the alias or type does not resolve; make `CheckedGraph::operation_frame` take its output; update `resolve_frame`, its two entry paths, `execute/frame.rs::operation_name` and the test at `spine/clause/tests/frame_replay.rs:85` | an inherited operation selects its declaring frame through the resolver; a formatted string no longer type-checks as a selection; TC-514 and TC-515 pass unchanged | QSL |
| TK-2 | G-2: `replay_frame` checks the envelope's `clause_node` and `occurrence_key` against the payload; the frame envelope's members as PF-4 states; amend FR-116 | a frame envelope whose `clause_node` or `occurrence_key` differs from the payload refuses `stale_dependency`/`revision-mismatch` naming both, before recompiling; TC-515 extended | QSL |
| TK-3 | G-3: delete `check::identity::{Frame, FrameSubjects, ResolvedFrameSubjects}`; re-home TC-248 onto `CheckedOperationFrame` and `OperationEffect` subject resolution (FR-088-AC-2); correct the stale `model/intake.rs:17-23` module doc | TC-248 passes against the live types; FR-088-AC-2 stays backed | QSL |
| TK-4 | Consume IR-370's `reaches_field` reference-edge check; TC-463 step 1 and TC-469 step 6 run without `#[ignore]` (FR-105-AC-3, FR-108-AC-6) | TC-463 step 1 and TC-469 step 6 pass | QSL |
| Q-1 | Surface spelling of the QSpec FR-353 abstraction relation in the shared grammar. Decided: QSpec FR-450 (STD-116) | QSpec | QSpec |
| Q-2 | `quire.checked-package/v2` node for the abstraction relation, carrying AR-2's keys and values. Decided: QSpec FR-451 (STD-116) | QSpec | QSpec |
| Q-3 | QSpec FR-290 claim-form row: remove "(a relation-family refinement gate)"; re-trace V1-TOOL-011 and V1-TOOL-012 from QSpec FR-177 and TC-208 to the #191 and #192 gate tests | QSpec | QSpec |
| Q-4 | QSpec FR-323 `selection` for a frame packet (a function or a frame operation), with CG's packet members (CG FR-024, agent-ix/quire-contract-codegen#50) | QSpec, CG | QSpec |
| Q-5 | Which profile layering V1-TOOL-012 covers for a compiler whose selections are fixed by its catalog, and which QSpec AD-003 edges V1 requires. Answered by QSpec FR-453: every layer header-selectable, all five edges | QSpec | QSpec |
| Q-6 | A catalog cause for an implementation-gap `unsupported_construct` refusal (QSL's `not-yet-implemented`) | QSpec | QSpec |
| Q-7 | QSpec FR-353-AC-1: accept a frame binding keyed by the operation (declaring type key, operation identifier), with the frame and anchor identities derived from it, or amend AC-1. Decided: QSpec FR-353-AC-1 accepts the derived key (STD-121) | QSpec | QSpec |

Ticket-text corrections for the owner (all ticket text read as data):

- QSL-39 cites QSpec FR-250 where it means QSpec AD-003 and V1-TOOL-012.
- QSL-40 and QSL-39 name QSpec FR-177 as their normative contract; their
  gates test QSpec FR-290's operation-contract refinement row (RF-1).

### 7. Amendments made with this record

- ADR-013 R-06 and ADR-011 §1: the post-check name exception is PF-3's
  layer-6 entry selection.
- ADR-013 O-08: public type per PF-5.
- ADR-012 §3 `Relation` row: the gates emit no claim (RF-1); the check
  column is AR-3's refusals and AR-4's per-item unbound refusal, with no
  totality check.
- ADR-012 §13.5 Q210-3 `Relation` gates: RF-4's categories.
- ADR-012 §2, ADR-013 O-16 ("Why S6a admits no `Relation`") and FR-090: the
  refinement gates are test gates whose cases reach S6a as clause runs
  (RF-1).
- ADR-013 O-25 packet list: a frame packet's members per PF-4.
- FR-057 claim-form row for refinement between operation contracts,
  FR-057-AC-10 (that row is outside V1) and its Dependencies; TC-153 step 7
  and its expected results; the FR-057-AC-10 row of
  `spec/model-linking/tests.md` (RF-1).
- `spec/spec.md`: index row.
- 2026-10-01, with FR-340 to FR-345: RF-2's pair direction is the corpus's
  `prior`/`superseding` naming alone (the lexical revision-label comparison
  is deleted); RF-2 gains the corpus layout; a prior class `incomplete` or
  `unsupported` at
  any stage, not only `compile` or `select`, is unresolved, so a prior
  run that reaches a limit at `admit` or `evaluate` is never `not
  applicable`; RF-4 names the reached limit
  of an incomplete case.
- 2026-10-01, Q-5 answered (QSpec FR-453): RF-3 covers all five AD-003
  edges with header-selected layers (FR-110 layer selection) and the
  user-subset check; RF-6 resolves a header by identity alone.

### 8. Open dependencies and ticket edges

1. **Q-1 and Q-2 (QSpec)** block #198's feature slice and ARCH-G4 scenario 8.
   CG#84 waits on Q-2 and on IR-33.
2. **Q-5 (QSpec)** is answered by QSpec FR-453; #192 is unblocked.
3. **IR-339 and CG#49**: a frame record settles `unsupported` until each
   lands (PF-2).
4. **Linear edges** for the owner: QSL-39 blocks QSL-36 today, but #198 needs
   nothing from #192, so that edge can go; #192 reuses #191's report and
   classification scaffolding, so QSL-40 blocks QSL-39.

## Consequences

- #191 and #198's enablement slice can be built now with no further
  ownership decision, and so can #192 over FR-110's layer selection.
  #198's feature slice waits on Q-1 and Q-2.
- Protocol/frame needs no redesign. Three bounded defects (G-1 to G-3) are
  ticketed.
- Refinement gates produce test evidence only. A regression names its case by
  `RawSourceRef` and selection, and unsupported, incomplete and tool-failure
  results are never promoted to holds.
- Kani and Verus read one relation from the v2 package, and an unbound
  element refuses in QSL before any backend is asked.

## Alternatives Considered

- **Read #191 as one source compiled under two language editions.**
  Rejected. A build carries one definition of each edition and profile
  identity, and keeps no earlier one (ADR-014 N-4), so the prior side could
  only be a recorded outcome. QSL-40's "versioned spec pair" and QSpec#116's "spec versioning" name
  two revisions of one specification, which one build compiles.
- **Hand-authored parent outcomes for #192.** Rejected. No compile checks
  them, so the gate would test the author's expectation, not refusal
  preservation (QSpec FR-453).
- **Treat `unsupported_construct` as a refusal #192 preserves.** Rejected. A
  child profile admits forms its parent prohibits; QSpec AD-003 keeps
  "prohibited by the selected profile" apart from a typed refusal.
- **Key bindings by `EffectiveId`.** Rejected. QSpec FR-353 names the
  declaration key; `EffectiveId` is another domain (ADR-013 O-05).
- **Key frame bindings by the frame and anchor `NodeKey`s.** Rejected. Those
  nodes exist only for an operation a clause or attempt names (FR-105), so an
  operation named by nothing else could not be bound.
- **Resolve Rust paths in QSL.** Rejected. QSL compiles no Rust; only the
  generator that emits the code can resolve them.
- **Carry the relation outside the checked package** (a side file for CG).
  Rejected. Kani and Verus would then have a second authority, which QSpec
  FR-353 and ARCH-G4 forbid.
- **A `ReplaySelection` sum in QSL now.** Rejected. The selection is a QSpec
  FR-323 member that CG copies; changing it is Q-4. TK-2's consistency check
  removes the silent disagreement without a wire change.
- **Select a frame by the payload's `WireNodeId`s instead of its name.**
  Rejected. The name selects and the identities bind (PF-3), as ADR-013 OQ-5
  ruled for functions, so an inconsistent packet refuses naming both
  identities.
- **G-4, the observation reader's `format!("{}/{name}", owner.node)` member
  key.** Not a defect. It builds QSpec's `<owner>/<name>` member identity,
  which intake enforces for every member (`model/intake.rs:832-835`), so the
  derived key equals the admitted key by contract.

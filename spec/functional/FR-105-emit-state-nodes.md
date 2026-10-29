---
id: FR-105
title: "Emit state clause, operation anchor and frame nodes in checked-package/v2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-341
    type: depends_on
---
# FR-105: Emit state clause, operation anchor and frame nodes in checked-package/v2

## Description

When a checked graph holds state clauses (FR-104), S4 SHALL emit, all or
nothing, the `state`/`state_clause`, `state`/`operation_anchor` and
`state`/`frame` nodes below (ADR-012 §15.3, §15.4). S4 SHALL name every model
member by its declaring type's node (`model`/`object_type`, `model`/`process` or `model`/`state_machine`) and its
member name, as ADR-013 O-06, FR-094 and QSpec FR-322 "Model-owned members"
already do, and SHALL emit no `model`/`field_declaration` or
`model`/`operation_declaration` node. The package's `package_id` covers these
nodes.

Today QSL emits no `state` node. `NodeTag::State` appears only in the tag map
(`qsl-package/src/emit.rs:254`) and the declaration-absent rule
(`emit.rs:388-396`); `CheckedClauseKind::StateTransition` is identity
vocabulary with no emitter (`qsl-semantics/src/check/identity.rs:150-195`).
The IR vocabulary already decodes all five `StateForm`s
(`quire-contract-model`, `checked_package/v2/vocabulary.rs:238-246`).

The wire spellings of these bodies, the (object type, member name) frame
entry, `quire.op.model.reaches_field` and `quire.op.state.clause` are QSpec
changes that STD-111 carries. v2 is prerelease and QSpec revises it in place
(ADR-012 §12.1).

## Inputs

- A `CheckedGraph` with checked state clauses and the model correspondence
  (ADR-013 O-04, FR-088).

## Outputs

The nodes below, each keyed by FR-092/FR-093's preimage rules (the
application-node preimage when its body holds an application), with its
`dependencies` listing every node its body references except a
`dependency_reference` (FR-093-AC-12), and its occurrences in the package
source map (FR-095).

| Node | `semantic_type` | `body` | Occurrences |
| --- | --- | --- | --- |
| `state`/`state_clause` | the `Boolean` scalar type node | an application of `quire.op.state.clause` whose `member` is `{kind: "state_clause", clause: <kind>, execution_point?: <accepted ClauseBinding point>}` and whose arguments are, in order: `aggregate` of the parameter references (`self`, then `result` when present, then the operation's parameters in declared order), a `reference` to the anchor, and the condition term | one `claim` occurrence per declaration that checks to this node, ordinals 0, 1, ... in source order |
| `state`/`operation_anchor` | the declaring type's node (`model`/`object_type`, `model`/`process` or `model`/`state_machine`) | `aggregate[ binding("context", reference(declaring type node)), binding("operation", literal(text, "<operation name>")), binding("frame", reference(frame)), binding("execution_point", <accepted ClauseBinding point>)? ]` | one `anchor` occurrence per clause or protocol attempt (FR-114) that names the operation, ordinals in source order |
| `state`/`frame` | the declaring type's node (as the anchor's) | QSpec FR-340's frame term: `modifies`, `creates`, `deletes` and `deletes_scoped`, with each field `modifies` entry an (object type node, field name) pair carrying its `scope`, each `creates`/`deletes` entry an `object_type` node, and each `deletes_scoped` entry `receiver` or a reference parameter's name (FR-154) | `generated`, one occurrence per named operation, ordinals by ascending (declaring `DeclarationKey`, operation name as UTF-8 bytes) over the unit's named operations, never by source order (FR-104 "Requirements", SR-736 FND-008/FND-009): two operations whose frame node holds equal content still get two occurrences, ordered this way. Each occurrence carries one region: the span of the operation reference (the qualified operation name) in the first clause or protocol attempt, in source order, that names the operation |

- Exception for derived nodes. A derived `state_clause` node and a
  `state`/`transition` node (see "Boolean model-field transitions") have no
  declaration that checks to them, so the "one `claim` occurrence per
  declaration that checks to this node" rule above does not apply to them.
  Each carries one `claim` occurrence per contributing authored declaration,
  in source order, each with that declaration's region, because the V2 schema
  requires at least one occurrence. QSpec FR-341's wording of the same rule is
  to be reworded to admit them (QSL-owned follow-up, a QSpec STD ticket).
- `<kind>` is `invariant`, `constraint` (a state constraint, FR-158),
  `precondition` or `postcondition`. The anchor is the context's node
  (`model`/`object_type`, `model`/`process` or `model`/`state_machine`) for an
  invariant or a constraint, and the operation's `state`/`operation_anchor`
  node otherwise. An invariant and a constraint bind `self` alone.
- `execution_point`, when present, is the accepted
  `ClauseBinding.execution_point` encoded as the exact closed IR form:
  `{kind: "initialization", name}`, `{kind: "handler", name}`,
  `{kind: "pre", operation}` or `{kind: "post", operation}`. The value is
  part of the state-clause application member and of the operation-anchor
  body, so each contributes to its node preimage and identity. The
  `state_clause` member omits it only when the checked clause has no point;
  an operation anchor omits it only when no clause binding supplies a point
  (for example, a protocol-attempt-only anchor). An operation clause's point
  matches between its clause and anchor. Different points on otherwise equal
  clauses or anchors produce different node identities.
- `self`, `result` and each operation parameter are `value`/`parameter`
  nodes, as a function's parameters are (FR-092). `result` is present only in
  a postcondition of an operation with a result.
- The condition is the checked Boolean body lowered by FR-093 exactly as a
  `pure_function` body is lowered. A field read lowers as the `deref` and
  member application FR-093 gives `deref(self).f`, whose member is
  `{kind: "field", declaration: <object_type node>, name: "<field>"}`.
  `pre(e)` lowers as `quire.op.state.pre`. `reaches(a, b, edge)` lowers as a
  `quire.op.model.reaches_field` application whose member is
  `{kind: "field", declaration: <object_type node>, name: "<edge>"}`. A `reaches`
  over a relationship end lowers by FR-365 as a `quire.op.model.reaches`
  application whose member is a `relationship_end`.
- The text literal's `type` is the `text` scalar type node.

## Behavior

- S4 SHALL emit the nodes above for every checked state clause, or none of
  them and a refusal (ADR-012 §2 "`package` is all-or-nothing"). The spine
  already refuses a compile with any omitted node
  (`qsl-replay/src/spine.rs:866-870`), and that rule SHALL hold for state
  nodes too.
- S4 SHALL emit one `state_clause` node per distinct clause node id. Two
  declarations that check to one node id (FR-104-AC-6) SHALL give one node
  with two `claim` occurrences, never two nodes with one id.
- S4 SHALL emit one `operation_anchor` per (declaring type, operation
  name, execution point), with absence as a distinct no-point value, however
  many clauses name the operation and from whichever subtype
  they name it, and none for an operation no clause and no protocol attempt
  (FR-114) names. The anchor's `frame` is the node of that operation's
  frame. A frame node's key hashes its body and its `semantic_type`, so
  operations of one declaring object type whose frames are equal have one
  frame node, referenced by each of their anchors and holding one `generated`
  occurrence per operation; a frame node is never referenced by the anchors of
  two object types, because its `semantic_type` is the anchor's context
  (QSpec FR-342).
  An operation inherited from `M::Base` and named as `M::Sub::op` anchors at
  `M::Base`. The anchor's context is the node of the declaring type: a
  `model`/`object_type` node for an object type, a `model`/`process` node for
  a process and a `model`/`state_machine` node for a state machine. An
  operation that a process declares and a clause or attempt names gets its
  anchor, its frame node and its frame record as any operation does, built
  from its `OperationEffect` (FR-103, FR-154), and the frame's
  `semantic_type` is the process node. A process operation's frame holds no
  receiver-scoped entry (FR-154), so its node holds `every-object` `modifies`
  entries and an empty or parameter-only `deletes_scoped`. An invariant on a
  process is anchored at the process node.
- The checker SHALL add `Invariant`, `Constraint`, `Precondition` and
  `Postcondition` to `CheckedClauseKind` (FR-088, as amended), each spelled
  with the node pair (`state`, `state_clause`), the clause operation
  `quire.op.state.clause` and its own kind member (`invariant`, `constraint`,
  `precondition`, `postcondition`), so the four spellings differ only in the
  kind member.
- The checker SHALL spell `StateTransition` with the node pair
  (`state`, `transition`), its QSpec form as STD-111 item 5 defines it (the
  `state`/`transition` body root is a `quire.op.state.transition`
  application). The fixed table in the TC-250 test
  `node_tag_and_semantic_form_mapping_matches_a_fixed_table_and_is_total_and_injective`
  (`qsl-semantics/src/check/identity.rs:619-631`), which asserts
  (`state`, `frame`) today, changes with this: the implementer SHALL update
  it to (`state`, `transition`) in the same change.
- The decoder of `CheckedClauseKind` SHALL map (`state`, `frame`) to no clause
  kind: a frame node's body is FR-340's frame term, which holds no clause
  application, so a frame under an `operation_anchor` never decodes as
  `StateTransition` or any other clause kind.
- S4 SHALL list a frame's `modifies`, `creates` and `deletes` entries in
  ascending order of (object type node reference, field name, scope with
  `every-object` first), and SHALL list
  each entry's object type node in the frame's `dependencies` (QSpec FR-340's
  dependency rule).
- S4 SHALL emit every field `modifies` entry with its `scope`, `receiver` or
  `every-object`, and `deletes_scoped` always, as FR-154 states: the
  `receiver` entry first, then `parameter` entries by name as UTF-8 bytes.
  A `deletes_scoped` entry names no node and adds no `dependencies` entry.
  The scope and the scoped deletes are part of the frame node's content, so
  they take part in its node id.
- S4 SHALL emit no `state`/`snapshot` node (ADR-012 §15.4). It SHALL emit a
  `state`/`transition` node only as "Boolean model-field transitions" below
  fixes, and for no other state clause.
- S4 SHALL emit bytes that are independent of any snapshot or invocation: the
  same unit and packages give the same `package_id` whatever observations a
  later run supplies.

### Boolean model-field transitions

QSpec FR-341 "State transition body" carries one `state`/`transition` node per
(`anchor`, `subject`) pair, binding at most one `pre` and exactly one `post`
`state_clause` node by reference, while FR-104 lets an operation hold any
number of preconditions and postconditions. S4 SHALL close that gap without a
QSpec or IR change, as follows. Every authored clause keeps its own
`state_clause` node and its own occurrences, so authored conditions and
diagnostics are unchanged. The transition is a derived node, not a source
construct: it adds nothing to the checked graph, the in-process name table or
the `requirements` hook, and no source clause refers to it.

- **Eligible subject.** For an operation anchor `A` (context `C`, operation
  `op`) and a field `F`, S4 SHALL emit one `state`/`transition` node for
  (`A`, `F`) exactly when all of these hold: `F` is a required-presence,
  single-valued `Boolean` field of `C` (not `Option<Boolean>`, not a
  collection) that `op`'s frame `modifies` with a field entry; every parameter
  of `op` is `Boolean` (QSpec FR-341 `inputs`); the package holds the no-point
  operation anchor of (`C`, `op`) and no `precondition` or `postcondition` of
  `op` carries an execution point (a protocol attempt never carries one and
  is not a clause here); no `precondition` or `postcondition` of `op` names
  the operation through a proper subtype of the anchor's context (`on
  M::Sub::op`), because FR-120 applies such a clause only to receivers
  conforming to the subtype, and a transition over the base anchor would claim
  more than the contract gives; the package's lock selects the protocol
  profile (QSpec FR-341 `laws`); and at least one postcondition of `op`
  constrains `F`. A postcondition constrains `F` when its condition holds the
  subject node itself, the `quire.op.record.project` over `deref(self)` that
  FR-093 lowers `self.F` to, outside every `quire.op.state.pre` application.
  A read that lowers to another node does not constrain `F`: `self.peer.on`,
  `let s = self in s.on` (the binder `s` is a parameter node of its own) and
  `result.on` are not constraining, and a read of `F` inside `pre(e)` is not.
- **Not eligible: not emitted.** An (`A`, `F`) for which any condition above
  fails gets no transition. The compile does not refuse, S3 records nothing,
  and every authored clause is emitted as FR-105 states above. An optional or
  multi-valued Boolean field, a field of another scalar type, an operation
  with a non-Boolean or reference parameter, a clause with an execution
  point, a clause on a subtype, and a field no postcondition constrains are all outside the
  transition vocabulary and keep only their clause nodes. A package with no
  eligible pair holds no `state`/`transition` node, as before.
- **`post`.** Let `Q` be the postconditions of `op` that constrain `F`. Count
  `Q` by distinct node id, not by declaration: two declarations that check to
  one node (FR-104-AC-6) are one clause. When `Q` holds one clause, `post`
  references that clause's node. When `Q` holds
  two or more, S4 SHALL emit one derived `state`/`state_clause` node of kind
  `postcondition` on `A` and `post` references it. Its parameter aggregate and
  anchor are those of the clauses of `Q`, and its condition is the left fold of
  `quire.op.boolean.and` over the conditions of `Q` in ascending clause-name
  order (UTF-8 bytes; a node shared by several declarations sorts by its least
  clause name), the order FR-107 evaluates postconditions in.
- **`pre`.** Let `R` be every precondition of `op`, not only those that read
  `F`: the transition states that `op` carries a state satisfying the
  operation's whole precondition to a state satisfying `post`, and a `pre` that
  omitted a precondition would claim more than the contract gives. `R` is counted by distinct node id as `Q` is. `pre` is the
  empty aggregate when `R` is empty, a reference to the clause's node when `R`
  holds one clause, and otherwise a reference to one derived
  `state`/`state_clause` node of kind `precondition` built from `R` as the
  derived postcondition is built from `Q`.
- **Exactness.** The `and` of the conditions is exact, and is checked
  structurally: the derived condition is the left fold of
  `quire.op.boolean.and` whose operands reference the contributing authored
  conditions' expression nodes in the order above, so it has no verdict of its
  own beyond the conjunction of theirs. FR-107 holds an
  operation's preconditions true only when every one is true, and its
  postconditions true only when every one is, so the conjunction of `R` is the
  operation's precondition and the conjunction of `Q` is implied by the
  operation's postcondition. The transition, true when `op` carries a state
  satisfying `pre` to one satisfying `post` (QSpec FR-341), is then implied by
  the authored contract and states nothing it does not. `Q` is the clauses
  that constrain `F`, so `post` may be weaker than the operation's complete
  postcondition, and a consumer SHALL NOT read it as complete (QSpec FR-341
  "Several postconditions and preconditions on one operation").
- **Derived clause nodes.** A derived `state_clause` node is keyed as any
  other (FR-093's application preimage) and has no `declaration`. Two
  subjects with the same `R` share one derived precondition node, because the
  node is content-keyed. A derived node's id never equals an authored node's: `R` holds every
  precondition of `op`, and any authored postcondition holding the fold holds
  the subject node and so is in `Q`, so the left fold over a set that contains
  an authored node `X` is never `X`, and a different anchor, kind or execution
  point changes the id or makes the pair ineligible. A derived node carries one
  `claim` occurrence per contributing authored clause declaration, in source
  order, each with that declaration's region (the Outputs exception above).
  A region may then be claimed by two nodes: FR-095's locus rule maps an
  occurrence key (node id, role, ordinal) to regions only, and QSpec FR-322
  allows the regions of different occurrences to nest or overlap. The derived node is not a name in the in-process table: `run` selects by authored
  clause name only (FR-109).
- **Transition node.** `anchor` is the no-point operation anchor, `subject` is
  the `expression`/`query` node FR-093 lowers `deref(self).F` to, `inputs`
  are the operation's parameters in declared order, and the application
  carries the protocol-profile law the lock selects. The node has no
  `declaration`; it carries one `claim` occurrence per declaration of a clause
  of `Q`, in source order, each with that declaration's region. S4 emits at
  most one transition per (`A`, `F`), in line with QSpec's uniqueness refusal.
- **All or nothing.** The transition and derived nodes are part of the
  package's all-or-nothing emission (above): a fault on any of them refuses the
  compile and emits no state node.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-105-AC-1 | The ConfigVersion unit of FR-104-AC-1 emits exactly three `state_clause` nodes (kinds `invariant`, `invariant`, `postcondition`), one `operation_anchor` (context `ConfigVersion`, operation `"attemptUpdate"`) and one `frame`, whose `modifies` is exactly (`ConfigVersion` node, `versionNumber`) with scope `every-object` and whose `creates`, `deletes` and `deletes_scoped` are empty. The package holds no `model`/`field_declaration`, `model`/`operation_declaration`, `state`/`snapshot` or `state`/`transition` node. | Test (TC-462); covered |
| FR-105-AC-2 | Each emitted node's body, `semantic_type`, `dependencies` and occurrences match the Outputs table; `VersionUnchanged`'s condition holds a `quire.op.state.pre` application over the field read of `versionNumber`; `NoCycle`'s holds a `quire.op.model.reaches_field` application whose member is `{kind: "field", declaration: <ConfigVersion node>, name: "parent"}`, the same member shape `ParentOrder`'s `self.parent` read carries. | Test (TC-462); covered |
| FR-105-AC-3 | The emitted package passes QSL's I2 read (`qsl-package` `checked_v2` reader), including its frame step, and its recomputed `package_id` equals the emitted one. | Test (TC-463) |
| FR-105-AC-4 | Compiling the unit twice gives identical bytes. Renaming `ParentOrder` changes no node id; changing its `<` to `<=` changes its `state_clause` node id and the `package_id`; adding `ParentOrder2` with `ParentOrder`'s body adds no node and a second `claim` occurrence (ordinal 1) to `ParentOrder`'s node; adding a second `post` clause on `attemptUpdate` adds one `state_clause` node and no second anchor or frame. Over a package where `Sub` specializes `ConfigVersion`, `pre A ... on Config::ConfigVersion::attemptUpdate` and `pre B ... on Config::Sub::attemptUpdate` share one anchor whose context is `ConfigVersion`. | Test (TC-463); covered |
| FR-105-AC-5 | `CheckedClauseKind`'s mapping is total over all eight variants: forward, each variant gives one (node pair, clause operation, kind member) triple and no two variants give the same triple; backward, each triple decodes to its variant; (`state`, `frame`) decodes to no variant; `StateTransition` gives (`state`, `transition`). A mutant that swaps two kind members, or deletes an arm, fails the test. | Test (TC-462); covered |
| FR-105-AC-6 | When the emitter cannot emit one of a clause's nodes (fault injected at the `frame` node), the compile refuses and emits no `state` node and no package bytes. | Test (TC-463 step 4); covered: `qsl-package`'s `a_fault_injected_partway_through_node_emission_writes_nothing` proves the all-or-nothing mechanism generically over the node-emission loop, and `qsl-replay`'s `a_fault_on_the_frame_node_refuses_the_whole_config_version_package` drives it at the ConfigVersion state clause's own `frame` node |
| FR-105-AC-7 | Over TC-1043's fixture R, where `all` and `same` have equal frames, the package holds one `state`/`frame` node for the two operations, whose body is vector W5 and which carries a `generated` occurrence for `all` and then one for `same`, and two `operation_anchor` nodes, one per operation, whose `frame` bindings reference that node; the package passes QSL's I2 read. | Test (TC-1043 step 2b) |
| FR-105-AC-8 | Compiling FR-104-AC-10's `P` and `Q` yields exactly one `operation_anchor` node for `go`, whose `context` is the `Flow` `model`/`process` node and whose operation is the text literal `"go"`, one `frame` node whose `modifies` is exactly (`Flow` node, `n`, scope `every-object`) with `creates`, `deletes` and `deletes_scoped` empty, one `operation-contract` frame record for `go`, and one `anchor` occurrence of the anchor per clause; `I` is anchored at the `Flow` node. A package with no clause or attempt naming `go` holds no anchor, frame node or frame record for `go`. | Test (TC-462) |
| FR-105-AC-9 | In FR-105-AC-1's package, the frame node's `generated` occurrence for `attemptUpdate` carries exactly one region, the byte span of `attemptUpdate`'s qualified operation reference in `VersionUnchanged`, the first clause naming it. Adding, before `VersionUnchanged`, a second `post` clause naming `attemptUpdate` moves the region to that clause's reference and leaves the occurrence's (node id, role, ordinal) unchanged. Over TC-1043's fixture R, the occurrences for `all` and `same` carry the span of each operation's reference in the first clause naming it. A `generated` frame occurrence with no region refuses `UnlocatedOccurrence` (FR-062). | Test (TC-462) |
| FR-105-AC-10 | Adding `constraint Low using v on Config::ConfigVersion at current { self.versionNumber <= 1 }` to the ConfigVersion unit of FR-104-AC-1 adds exactly one `state_clause` node, whose body is a `quire.op.state.clause` application with kind member `{kind: "state_clause", clause: "constraint"}`, a parameter aggregate holding only the `self` `value`/`parameter` node, the `ConfigVersion` `model`/`object_type` node as anchor, and the lowered condition; it has one `claim` occurrence and adds no `operation_anchor` or `frame` node. Replacing `constraint` with `invariant` in that declaration gives a node with a different node id, and the package passes QSL's I2 read in both cases. `(state, state_clause, quire.op.state.clause, constraint)` decodes to `Constraint`. | Test (TC-462) |
| FR-105-AC-11 | With only `attempt Update by R on Config::ConfigVersion::attemptUpdate contracts []` naming `attemptUpdate`, the frame's `generated` occurrence has one region: the qualified operation reference `Config::ConfigVersion::attemptUpdate` in the attempt, not the attempt name `Update`. When that attempt and `VersionUnchanged` both name `attemptUpdate`, the occurrence's region is the operation reference in whichever declaration appears first in source order: the attempt when it precedes the clause, and the clause when it follows it. Reordering those declarations leaves the occurrence's `(node id, role, ordinal)` unchanged. | Test (TC-462) |
| FR-105-AC-12 | The accepted `ClauseBinding.execution_point` is emitted in the `state_clause` operation member and, for an operation clause, the referenced `operation_anchor` body using the closed `Initialization{name}`, `Handler{name}`, `Pre{operation}` or `Post{operation}` shape; both carrier locations participate in their node preimages. The names and variant match the checked clause, the clause and anchor points agree, different points create different node identities, and a protocol-attempt-only anchor omits the point. QSL's V2 read admits the exact bytes and rejects altered, inapplicable or mismatched points. | Test (TC-462, TC-463) |
| FR-105-AC-13 | Over a fixture whose lock selects the protocol profile, with object type `Switch`, required Boolean fields `on` and `lit`, an optional `Option<Boolean>` field `armed`, an `Integer` field `count`, and operation `set(flag: Boolean)` whose frame modifies `on`, `lit`, `armed` and `count`, with `pre P1 { flag or self.on }`, `post Q1 { self.on }`, and postconditions `Qa { self.armed = pre(self.armed) }`, `Qc { self.count = pre(self.count) }` and `Qs` reading a `Sequence<Boolean>` field `seq` that `set` also modifies, the package holds exactly one `state`/`transition` node for (`set`, `on`): `anchor` the no-point anchor, `subject` the `deref(self).on` projection, `inputs` `[flag]`, `pre` a reference to `P1`'s node and `post` a reference to `Q1`'s node. It holds no derived `state_clause` node, no transition for `lit` (no postcondition constrains it), and none for `armed`, `count` or `seq` although a postcondition constrains each, and `P1` and `Q1` keep their own nodes and occurrences. The package passes QSL's I2 read. | Test (TC-1271); covered |
| FR-105-AC-14 | Adding `post Q2 { self.on = pre(self.on) or flag }` and `pre P2 { not flag or self.lit }` to AC-13's unit, the (`set`, `on`) transition's `post` references one derived `postcondition` node whose condition is `and(Q1's condition, Q2's condition)` in ascending clause-name order and whose anchor and parameter aggregate equal `Q1`'s, and its `pre` references one derived `precondition` node `and(P1, P2)`; `P1`, `P2`, `Q1` and `Q2` keep their own nodes; the package holds exactly one `state`/`transition` node; swapping the declaration order of `Q1` and `Q2` in source leaves every node id and the `package_id` unchanged. `Q1` and `P1` are each one distinct node in `Q` and `R` when a second declaration checks to the same node. A postcondition reading `on` only inside `pre(...)` is not in `Q`, and a precondition that does not read `on` is still in `R`. | Test (TC-1271); covered |
| FR-105-AC-15 | The conjunction is structurally exact: in AC-14's unit the derived `pre` condition is `quire.op.boolean.and` over `ref(P1's condition)` and `ref(P2's condition)` and the derived `post` condition is `quire.op.boolean.and` over `ref(Q1's condition)` and `ref(Q2's condition)`, in that order, with no other operand; with a third clause the fold is left-nested. The authored clauses' nodes, names, occurrences and `requirements` records are the same with and without the transition. | Test (TC-1271); covered |
| FR-105-AC-16 | Each of these gives no `state`/`transition` node, no derived node and no refusal, and leaves every authored clause node as before, and each carries a postcondition that would constrain the field were the variant eligible: `Switch.set2(n: Integer)` (a non-Boolean parameter) with `post { self.on }`; an operation whose frame does not modify `on`, with `post { self.on }`; `armed: Option<Boolean>`, a `Sequence<Boolean>` field `seq` and an `Integer` field `count`, each with a postcondition that reads it; a postcondition `{ self.peer.on }`, `{ let s = self in s.on }` or `{ result.on }`, or `{ pre(self.on) }`, alone (not constraining `on`); a `pre` or `post` of the operation carrying an execution point; a postcondition `post QS on M::SubSwitch::set { self.on }` on a proper subtype (AC-13's unit gains no transition for `on` once `QS` is added); a lock that selects no protocol profile. FR-105-AC-1's ConfigVersion package (no Boolean field) holds no `state`/`transition` node. | Test (TC-1271); covered |
| FR-105-AC-17 | Compiling AC-14's unit twice gives identical bytes; two eligible subjects `on` and `lit` with the same preconditions share one derived `precondition` node and give two transitions with different `post` and `subject`; each transition occurrence carries the region of a contributing postcondition declaration. A fault injected on the transition node or a derived node refuses the compile and emits no state node. | Test (TC-1271); covered |

## Dependencies

- FR-088 (clause kind, frame identity; amended by this PR), FR-092 to FR-095
  (keys, lowering, model-owned members, source map), FR-104 (checked
  clauses). FR-094 needs no amendment: no field or operation member gets a
  node of its own.
- STD-111 (QSpec): the `state_clause`, `operation_anchor` and `frame` body
  rules, the (object type, member name) frame entry,
  `quire.op.model.reaches_field` with the `field` member kind and its
  reference-edge constraint, `quire.op.state.clause` with its operator class
  and catalog entry, the `state_clause` `OperationMember` kind, and the node
  form rule (a `state`/`transition` body root is a `quire.op.state.transition`
  application; a frame carries no clause application). Only this FR's
  emission criteria and FR-108's reading of the emitted package wait on it;
  FR-102 to FR-104 and FR-106 to FR-109 read the in-process `CheckedPackage`.
- IR `lower` returns no form for any `state` node, so these nodes
  reach no backend. That is IR's own work (ADR-012 §8, §15.7).

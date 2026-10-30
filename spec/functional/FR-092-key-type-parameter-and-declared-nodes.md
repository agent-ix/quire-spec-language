---
id: FR-092
title: "Build type, parameter, value and declared function nodes with structural identity"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: traces_to
---
# FR-092: Build type, parameter, value and declared function nodes with structural identity

## Description

`check` builds the checked type, parameter, value, function and protocol nodes
below. QSpec FR-322 (quire-specification main at 686e49e8) owns their
`node_id`: the closed `{package, path, ordinal}` object, compared
componentwise and never hashed. Its `package` is the containing package's
name, a declared node's path is its declaration path, and a derived node's
path and producer-assigned ordinal follow FR-322's structural node identity
rules. This requirement does not define an identity preimage or digest.

## Inputs

- The checked declarations, types, bodies, binders and bound protocol targets.
- The package name, source owners, graph dependencies and checking limits.

## Outputs

- Checked nodes with QSpec FR-322 structural ids, or typed refusal with no
  partial checked package.

## Behavior

### Node identity and ownership

`check` SHALL give every emitted node a QSpec FR-322 `node_id`. A declared
node carries its source owner and its declaration path. A derived node's id
uses the FR-322 path for its tag, semantic form and operands outside a
recursion group; inside a group FR-322's group rules apply. The producer
assigns an ordinal unique among nodes sharing one package and path. Equal
ids denote one node; two distinct nodes SHALL NOT reuse an id. Node bodies,
source locations and JSON bytes are not hashed to derive ids.

An anonymous builtin or composite type has no declaration or source owner.
A declared type carries its source owner. A source alias adds no type node.
The checked graph resolves every reference to the structural id of its
actual target. `check` SHALL preserve the exact owner and source-map region
of each declaration even when moving it between units does not change its
package and declaration path.

### Type nodes

`check` SHALL build the type node of each checked `ValueType` as follows.
Each binding is `{term: "binding", name, value}`; an aggregate lists bindings
or references in the order shown. Bounds are typed scalar literals, not
identity-preimage spellings.

| Checked type | `node_tag` / `semantic_form` | `semantic_type` | Body |
| --- | --- | --- | --- |
| `Boolean`, `Integer`, rational, decimal, text and float base types | `scalar_type` / corresponding scalar form | itself | empty aggregate |
| `Int[lo, hi]` | `bounded_domain` / `integer_range` | `Integer` | `min`, `max` bindings |
| rational, decimal, text and float bounds | `bounded_domain` / corresponding range or bounds form | base scalar | their declared bounds and mode/profile bindings |
| `Option<T>` | `composite_type` / `option` | itself | reference to `T` |
| a collection `K<T>` | `composite_type` / collection form | itself | reference to `T` |
| bounded collection `K<T>[min,max]` | `bounded_domain` / `collection_bounds` | `K<T>` | `min`, `max` bindings |
| declared record | `composite_type` / `record` | itself | fields in declaration order; an optional field has an `optional` group |
| declared tuple | `composite_type` / `tuple` | itself | element references in position order |

The type of a bounded form is its `bounded_domain` node, not its base. A
source declaration supplies its name and owner; a caller-supplied handle is
not a node id. Type construction and expression lowering walk arbitrary
nesting with an explicit heap stack bounded by the stage's node, byte and
work limits (FR-258); neither depth nor stack size changes the result.

### Parameters and functions

`check` SHALL build one `value`/`parameter` node per binder, with the
binder's checked semantic type, no declaration or owner, and an aggregate
body binding its name as text and level as an integer. A function's
parameters have levels `0` to `n-1` in order; a nested binder's level adds
its enclosing binders. A reference to a parameter denotes the enclosing
binder at that level with the same name and type.

A function node has `node_tag: function`, its result semantic type, the
source declaration and source owner. Its `semantic_form` is
`recursive_function` in a recursion group, otherwise `predicate` for a
predicate or `pure_function`. Its aggregate body binds `parameters` in
parameter order, `body` to the checked root expression, and `decreases` to
the measure when present. Unread parameters remain in the list. Its body
contains references, not an inline application. The root expression and
applications are separate nodes. A predicate with a recursion group is
refused under FR-146's missing-measure rule. Model-owned clause functions
carry FR-094's model owner and clause binding.

A recursion group is represented under QSpec FR-322's group rules. The
producer assigns each member's structural id independently of declaration
visit order and never derives an id from group order or an RFC 8785
preimage. Two groups with different declarations retain distinct member
ids; an application in one group does not collide with an application in
another merely because their bodies are equal.

### Protocol reference nodes

`check` SHALL build one `protocol`/`protocol_reference` node for references
to a bound local protocol. Its body is exactly `{term: "reference", target}`;
its semantic type equals the target's, and its dependencies contain exactly
the target id. It has no declaration or owner. Each source reference adds
an `expression` occurrence with its own region and source-map row;
occurrence ordinals follow source position and FR-480 canonical unit order.
A second occurrence does not change the carrier id or body. FR-322 defines
the derived carrier's structural path from the target id. An absent or stale
target refuses `missing_declaration`/`missing-name` and emits no carrier.

For a protocol declaration that checks, `check` SHALL build the
`protocol`/`protocol_clause` node with the FR-383 `quire.op.protocol.control`
body, selected law and thirteen clause arguments, Boolean semantic type,
source declaration and owner, and a declaration occurrence. Its structural
id follows FR-322; a reference resolves to it. Unsupported constructs and
missing profile selections retain FR-383 and FR-218's refusals.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-092-CON-1 | Only `check` assigns the structural id for a node it constructs; no caller-supplied key or identity preimage is accepted. | Design | Inspection |
| FR-092-CON-2 | The matches that select node forms and build checked type nodes have no catch-all arm (ADR-012 §5.1). | Design | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-092-AC-1 | `Boolean`, `Integer`, text, `Int[0,9]`, `Option<Int[0,9]>`, `Sequence<Int[0,9]>`, bounded sequence and bounded text produce the type-node tags, semantic forms, semantic types and bodies in the Type nodes table, each with a valid FR-322 structural id. | Test (TC-413) |
| FR-092-AC-2 | Two units of one package using `Int[0,9]` resolve the same anonymous type node. A declared `Point` in either unit carries that unit's source owner and declaration region; its id has the package name and `Point` declaration path. Two compiles of the same package give equal ids. | Test (TC-413) |
| FR-092-AC-3 | An alias `type Digit = Int[0,9]` adds no checked node; a parameter of `Digit` resolves to the `Int[0,9]` type node. | Test (TC-413) |
| FR-092-AC-4 | Functions with two parameters, no parameters, a `let` binder and an integer literal construct the parameter, function and literal nodes and references specified above. Their ids have FR-322 structural components and no preimage bytes or digest. | Test (TC-414) |
| FR-092-AC-5 | A function with an unread second parameter still lists both parameter references in order; a second function in the same package may reference the same parameter node where its structural id is equal. | Test (TC-414) |
| FR-092-AC-6 | A function body contains no inline application term. The `a and b` expression is a separate application node, and both it and the function have FR-322 structural ids. | Test (TC-414) |
| FR-092-AC-7 | On a 512 KiB stack, 100,000 nested `Option`s check under limits raised to fit, with the same structural type id as on an 8 MiB stack. At default limits a 30-record optional-field chain checks on a 2 MiB stack. Two otherwise equal recursive declarations named `f` and `g` have distinct declared ids and do not collide solely because their conditional bodies are equal. | Test (TC-413) |
| FR-092-AC-8 | A source enum and member carry their QSpec FR-322 structural ids, the declaration path and source owner; neither is assigned a nominal preimage key. | Test (TC-413) |
| FR-092-AC-9 | Rational and decimal bounded types, a tuple and two records differing only by optional-field syntax produce the distinct type-node shapes above. `rational(1,2)` and `rational(2,4)` lower to the same reduced rational value. All nodes have structural ids. | Test (TC-413) |
| FR-092-AC-10 | A function with `decreases(x)` binds the measure to a reference to its parameter node and retains parameter order. | Test (TC-414) |
| FR-092-AC-11 | Recursive function and record groups (`f`, `List`, `Tree`, `ping` and `pong`) retain their references, semantic-type links and QSpec FR-322 group membership; reversing `ping` and `pong` declarations leaves their declared structural ids unchanged. Repeated references to one call use one node id where their package, path and ordinal are equal. | Test (TC-413) |
| FR-092-AC-12 | The id of declared `Point` derives from its package, declaration path and producer ordinal, independent of any caller handle; no checked node stores a caller handle as its id. | Test (TC-413) |
| FR-092-AC-13 | A predicate `Positive` and pure function `Positive` have distinct semantic forms. A source enum `Status` and member `READY` have their declared node tags, semantic forms, declaration paths and source owners. | Test (TC-481) |
| FR-092-AC-14 | Parameters with `Int` bounds through unsigned-64 and signed-128 extremes construct valid bounded-domain type nodes with exact integer bounds, without magnitude-based identity refusal; their ids satisfy FR-322's structural ordinal bound. | Test (TC-910) |
| FR-092-AC-15 | Withdrawn: QSpec FR-322 replaced RFC 8785 preimage-counter spelling with a bounded structural `node_id.ordinal`; no counter-encoding key oracle remains. | Inspection |
| FR-092-AC-16 | A unit that declares `protocol Fill` and holds `protocol Fill` as an expression emits one `protocol`/`protocol_reference` node whose body is exactly `{term: "reference", target: Fill's node_id}`, whose `semantic_type` equals `Fill`'s, and whose `dependencies` are exactly that id. It has one `expression` occurrence at the reference's region (ordinal 0), its own source-map row, and no `declaration`, `owner` or recursion group. The graph has one node collection and no protocol-reference collection. The IR V2 reader admits it as `ProtocolForm::ProtocolReference`. | Test (TC-1021) |
| FR-092-AC-17 | Repeating a reference to one protocol adds an occurrence and source-map row without changing the carrier id, body or dependencies. Changing its bound target changes the target operand in its FR-322 structural path and gives a distinct id. Removing the second occurrence leaves the carrier id unchanged. | Test (TC-1021) |
| FR-092-AC-18 | A unit that declares `protocol Fill` made only of FR-383's emitted constructs (static and replicated roles, channels and queues, `sequence`/`branch`/`case` controls, binders, captures, an `each` guard with no declaration read, a finish with no declaration read, and scheduling) emits one `protocol`/`protocol_clause` node. Its body is a `quire.op.protocol.control` application with the selected `protocol_profile` law, the `protocol` `profile_operator` member and FR-383's thirteen clause arguments; it has Boolean semantic type and one `declaration` occurrence at the declaration's region. The IR V2 reader admits it, and a reference to `Fill` resolves to its structural id. If no profile is selected, QSpec FR-322-AC-22 admits no `protocol_control` application: QSL emits no protocol node and no refusal for the declaration, while a reference to it refuses `missing_declaration`/`missing-selection` (FR-218-AC-8). An FR-383 construct not yet emitted refuses `unsupported_construct`/`not-yet-implemented` at that construct, names it and the ticket that writes it, and writes no clause node. | Test (TC-1021) |

## Dependencies

- QSpec FR-322, `Structural node identity` and the checked-package V2 reader
  contract, at quire-specification main 686e49e8. The authoritative file is
  read in place; it is not copied into this repository.
- FR-091, FR-093, FR-094, FR-218, FR-258, FR-383 and ADR-011/012 for
  checked graph construction, source owners, type and expression lowering,
  protocol emission, resource limits and exhaustive dispatch.

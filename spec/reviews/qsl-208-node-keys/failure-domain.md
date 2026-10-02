---
id: SR-525
title: "Failure-domain review of the QSL-208 node-key and expression-lowering requirements"
type: SpecReview
analysis: failure-domain
scope: "Commit d96591be: FR-092 and FR-093 node model, preimages, binder levels, recursion groups, occurrences and ownership split"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

These parts of the design hold:

- **Domain separation.** Every preimage is hashed into
  `quire.checked-semantic-node/v1`, and the constant `version` member keeps
  the structural-node, application-node and nominal preimages apart.
- **Parameters.** A parameter node carries no owner and no function, so a
  function key can depend on its parameters without a cycle.
- **Binder levels.** They are well defined because the checker refuses a
  shadowing name (`bind`, `check.rs:573-588`). They are computed from scope
  depth, not from the checker's monotonic `Slot`, and FR-093-AC-5 would catch
  a slot-based level.
- **Self-typed types.** A `null` `semantic_type` breaks the self-reference of
  `scalar_type` and `composite_type` nodes.

Splitting a function body into undeclared expression nodes has one failure
that the application-node model did not have. Inside a recursion group,
`group_reference` erases which group member a node points at, and expression
nodes carry no `declaration` or `owner` to restore it. Two structurally
identical recursion groups therefore key their expression nodes identically.
The other findings are nodes with no source occurrence, identity that ignores
field optionality, and a depth bound on the wrong walk.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Expression nodes in a recursion group collide. Take `function f … decreases n { if n = 0 then 0 else f(n - 1) }` and `g`, the same function under another name. Each forms a recursion group {function node, `if` node, call node}. In each group, the call's body is `[group_reference(ord), ref(n - 1)]` with `recursion {3, ord}`. Only the function nodes differ, by `declaration` and `owner`, and the `if` and call nodes carry neither. So `f`'s and `g`'s `if` and call nodes get equal keys but different dependencies. In one package this gives duplicate node ids, which FR-322 refuses. Across owners it gives equal `NodeKey`s for different code. The same happens to an anonymous `Option<R>` inside a self-referential record `R` (the checker's `contains_ieee` already guards composite cycles). FR-092 also does not say which nodes form a function's recursion group. Fix: say that the group is the call-graph SCC together with every expression and anonymous type node on its cycles. Then either key an in-group expression or anonymous type node with the group's declared members' `{declaration, owner}` (for example, a `group` member listing them in ordinal order), or keep in-group bodies inline in the declared node. Add a vector pair `f`/`g` that shows distinct keys. | FR-092:85-86, :184-186; FR-093:66-69; `check/refusal.rs` `UnprovedDecrease`; `value/declaration.rs:558-575` |
| FND-002 | medium | Some type nodes have no source occurrence. Every parameter node's `name` literal is typed by the text scalar node (T3), and its `level` literal by `Integer` (T2). Every bounded type's bound literals are also typed by T2. So a package with no `Text` in source still has T3, and T3 has no `type` or `expression` occurrence. FR-322 says "every node has one or more exact source occurrences". FR-093 gives only `expression` and `declaration` occurrences. Fix: state that a node reached only through such an implicit reference gets a `generated` occurrence at the region of the construct that required it (the binder or the bounded type), and add the check to TC-416. | FR-092:114-116, :163-164; FR-093:67-72; FR-322.md:62, :192-196 |
| FND-003 | medium | Optionality is not in a record's key. A field `f?: T` binds `f` to the `Option<T>` node, which is the same binding a required field `f: Option<T>` gets. The two declarations get one key, yet only the first admits an omitted slot (`RecordSlot::Absent`). A revision that switches between them keeps the node id and changes which values are valid. Fix: add the optional marker to the field binding, for example a binding value `aggregate{[reference(Option<T>), literal true]}` or a separate `optional` binding list, and add a vector with an optional field. | FR-092:129; FR-093:89; `check/ir.rs:113-120` |
| FND-004 | medium | The depth bound is on the walk that cannot get deep. Under FR-093, every checked expression and every type constructor is its own node, and a body references its operands. So a node body has bounded depth (application, then binding, then binding, then reference), and FR-092-AC-7's body "nested deeper than `MAX_CHECKING_DEPTH` terms" cannot be built through `check`. The unbounded walk is the lowering: a key needs its referenced nodes' keys first, so a checked tree of depth D gives a dependency chain of length D. Fix: bound, or make iterative, the lowering walk that computes keys children-first, and state its refusal. Keep the per-body bound as a guard, and move AC-7 to the lowering: a checked tree at the limit is keyed, and one deeper is refused. | FR-092:106-108, :414; FR-093:64-72; TC-413 step 5 |
| FND-005 | low | Occurrence ordinals across units are not defined. A shared node's `expression` occurrences get ordinals "in source order", but a package has several source units and no order among them is stated. Since P1, L1 and T1 are shared across functions and units, their ordinals, and so the source-map keys, depend on the order in which units are visited. Fix: order occurrences by (source identity, byte start), or by the unit order FR-091's assembler fixes. | FR-093:69-72; ADR-013 O-07 |
| FND-006 | low | Equal values get different nodes. An omitted optional slot has no binding. An explicit `null` gets a `none` literal binding. If QSL evaluates the two as the same record value, one value has two node ids, and equality over node ids (structurally identical means the same id) fails. The lowering of `flatMap` to `Flatten` over `Map` gives a different node than the catalogued `quire.op.collection.flat_map` for the same operation. Fix: give omitted and `null` slots one spelling, or state that they are different values. Lower `flatMap` to `flat_map`, or state that QSL never emits `flat_map`. | FR-093:89, :132; catalog `quire.op.collection.flat_map` |

## Resolution

FND-001: FR-092 defines a recursion group as a strongly connected component,
refuses a package in which two distinct nodes share a preimage with
`unknown_required_feature`/`unsupported-feature` (FR-092-AC-7), and records the
design question as FR-092-OQ-1 and in QC-24, pending QSpec's recursion
preimage (IR-242). The fail-closed refusal is the fix in this PR; the key
design is open. FND-002: a node that no region denotes has a `generated`
occurrence (FR-093-AC-9, TC-416). FND-003: an optional field binds
`binding{name: "optional"}`; D3 and D4 differ. FND-004: the depth bound
covers type-node building and the lowering walk, and AC-7 tests it through a
configured limit. FND-005: ordinals follow (source document, region start,
region end). FND-006: an omitted optional slot and `null` lower to one `none`
literal; `flatMap` lowers to `quire.op.collection.flat_map`.

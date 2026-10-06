---
id: SR-1349
title: "Spec review of quire-spec-language PR #648: FR-093 node owner emission (QSL-638)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@aabccf9cbe72841d745bfbf9206314343b358cd1; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (Who builds the lowering, AC-7, AC-21, AC-22, Dependencies, Status); spec/functional/FR-092-key-type-parameter-and-declared-nodes.md (one sentence); spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md (one sentence); spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md (steps 12-13, expected results, Status); context: qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/node_key/mod.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs; paired agent-ix/quire-specification#190 at 0b4f873 (FR-322 owner member)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
---
## Summary

Ticket: QSL-638. The brief named head 3260821ec. The branch has since gained
one commit, aabccf9cb, which only adds to the FR-093 Status note that IR adds
`owner` to its `CheckedSemanticNodeV2` first. This review is at aabccf9cb.

Lenses applied: the base checklist, integrity (consistency with the paired
QSpec PR), failure-domain (which nodes carry an owner), and testability.

**Consistency with QSpec #190.** The owner shape matches: `SourceOwner`
`{kind: "source", authority, identity}` and `ModelOwner`
`{kind: "model", identity, node}` with no `version`. The set of nodes that
carry an owner also matches. FR-093 defines it by preimage: a node carries an
owner when its `quire.structural-node/v1` preimage has one. QSpec's
OwnerPresenceRule defines it by node shape: a structural node with a
declaration, or an undeclared model/object_type, model/systems_interface,
relation/relationship or function node. For every node QSL writes, the two
rules pick the same nodes. The identity_projection entry and the lock joins
(`sources`, `model_selections`) also agree.

**Does the emitter have the owner for every node kind?** Yes. Every checked
`SemanticNode` already holds its owner (`SemanticNode::owner()`,
lowering.rs:224-229):
- `Lowering::insert` sets a `SourceOwner` exactly when the node has a
  declaration, and that owner is the unit's `RawSourceRef`
  (authority, identity). The emitted lock's `sources` always holds that
  pair.
- `insert_owned` sets a `ModelOwner` on clause functions. Model declaration
  nodes get one from `model_node_content`, and only for
  model/object_type, model/systems_interface and relation/relationship,
  the same three forms QSpec lists.
- Every undeclared function is a clause function (lowering.rs:2604-2631),
  so QSpec's "undeclared `function` node → ModelOwner" row holds.
- Nominal enum, dimension and unit nodes are inserted with `owner: None`.
- `typed_preimage` refuses an owned application node (`OwnedApplication`).
- Recursion-group drafts copy the owner through.

So the emitter only has to copy `node.owner()` onto the wire node and its
projection entry. What blocks that is the wire type: the emitter builds
IR's `CheckedSemanticNodeV2`, and that type has no `owner` field at
quire-contract-ir main a4aa02d. That is the IR-first order the new Status
sentence records.

The FR-092 and FR-094 sentences are correct. No hash, pin or tracking record
is added: `owner` is identity content.

## Verdict

Changes requested, spec text only. The FR-093 statement is right and
implementable. Two parts of the test spec are wrong. TC-416 step 12's
expected result says no other node carries an owner, but the test's own
package contains other owner-bearing nodes, so a correct emitter fails it.
AC-22 says its nodes are QSpec's two-owner fixture nodes, but they cannot
be, because the qualified names differ. Three more findings are
testability and traceability gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-416 step 12's expected result says "exactly the declared structural nodes (`P`, `Tree`), M1 and the clause functions carry `owner` … no other node carries one". Step 12's own package also holds owner-bearing nodes that this list leaves out. The declared "function over a `Reference<M::Order>` parameter" is a declared structural node, and `Lowering::insert` gives it the `SourceOwner` (`a`, `u`). FR-094-AC-5's fixtures include `Sub.size` with receiver `Reference<M::Sub>`, which brings in `Sub`'s model declaration node M5 with a `ModelOwner`. A correct emitter fails this expected result. Fix: state the rule instead of a list ("a node carries `owner` exactly when its preimage does"), or list every owner-bearing node: the declared function(s), M1, M5 and any other model node, and C1, C2, C4, C5 and C6. | spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:151-155 |
| FND-002 | high | FR-093-AC-22 ends with "These are the nodes of QSpec's `positive-two-owners-a.json` and `positive-two-owners-b.json` (QSpec FR-322-AC-53)". QSL lowers the AC's source, `record Point { x: Integer; }`, to `declaration.qualified_name` `["Point"]` (FR-092 vector D1; `qualified_name` splits only the declared name). QSpec FR-322-AC-53 at #190 0b4f873 declares those records under a namespaced qualified name, and TC-233 now says positive fixture nodes are real QSL emitter output. `declaration` is in the structural preimage, so QSL's `Point`/`List` ids cannot equal the fixtures', and a test that checks this sentence fails. Fix it on one side: make the AC-22 source produce the fixtures' qualified names, or have QSpec AC-53 use the unqualified `Point`/`List` that QSL emits. Otherwise drop the identity claim and keep only the differ/equal comparisons. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:731 |
| FND-003 | medium | FR-093-AC-21 and TC-416 step 12 need an emitted package that holds FR-094-AC-5's clause functions, and IR's reader must admit it. Those clause functions exist only through `checked_dispatch_operation` with a test-supplied `OperationClauses` table: no QSL source form produces one (FR-094-AC-5, checked_dispatch.rs:19-28). No existing test emits a package that holds a clause function. Such a package has no source unit, so where its generated occurrences are placed is unverified. The emitter refuses an unplaced occurrence (`UnlocatedOccurrence`). As written, the step does not say how to build the package, and the AC may not be testable. Fix: name the route in step 12 (for example `CheckedPackage::link` over the dispatch-built declarations, then `emit_checked`) and the source unit it is placed against. Alternatively, drop clause functions from AC-21 and back their owner with a TC-418 preimage-level check. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:730; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:86-94 |
| FND-004 | medium | FR-093-AC-21 says the "enum nodes carry none", but none of its sources (`P`, `Tree`, the `Reference<M::Order>` function, FR-094-AC-5's clause functions) declares an enum. So that clause can never fail, and no nominal node is exercised at all. The FR-093 statement explicitly excludes nominal nodes. QSpec FR-322-AC-51 refuses an `owner` on a nominal enum node. A nominal enum declaration node does carry a `declaration`, so it is the most likely place for an emitter to copy an owner wrongly. Fix: add `ordered enum Status { READY, DONE }` (or another enum) to step 12's source, and assert that its declaration and member nodes carry no `owner`. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:730 |
| FND-005 | medium | FR-093-AC-7 now says the structural key is recomputed "with the owner read from the node's own `owner` member". Its existing tests (qsl-package/src/emit/tests.rs `every_written_node_recomputes_to_its_node_id`, with the owner hardcoded as (`a`, `u`) at tests.rs:465) do not do that, and cannot until the emitter writes `owner`. TC-416 Status still says only steps 12 and 13 are unimplemented, so AC-7's trace tags claim a criterion the code does not meet. AC-21 already states the from-the-wire recompute. Fix: revert the AC-7 wording, or list AC-7 as unmet in TC-416 Status and the FR-093 Status note. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:716; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:183-184 |
| FND-006 | low | The Dependencies bullet ("IR's v2 node type carries it once IR adopts that member") and the new Status sentence describe the IR-first order but do not name its ticket. IR-627 is the IR side that QSL-638's ruling names. AC-21 and AC-22 ("IR's v2 reader admits") cannot pass before it lands. Fix: cite IR-627 in both places. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:759-761, 816-819 |
| FND-007 | low | In "Who builds the lowering", the existing sentence "A recursion group's members carry one `recursion_group` label …" is now attached to the end of the new owner paragraph (line 361), so the recursion-group rule reads as part of the owner rule. Fix: start a new paragraph at "A recursion group's members". | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:361 |

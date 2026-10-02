---
id: SR-1207
title: "Code review of quire-spec-language PR #594: lower declared unit and dimension nodes (QTY1)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a9e332b892cdb40a8d6ec9e8e16f2770a410cb33; PR #594 diff against origin/main: qsl-package/src/emit.rs, qsl-package/src/emit/extent_agreement.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/admission_corpus.rs, qsl-semantics/src/check/assemble/units.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/lowering/model/tests.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/value/unit.rs, quire-semantic-value/src/{location.rs,quantity.rs,unit.rs}, spec FR-062/091/093/094/097, TC-160/416/419/440"
review_set: subset
---
# Code review of quire-spec-language PR #594

## Summary

Ticket: QSL-247. `admit_unit_graph` now keeps each dimension's and unit's
qualified name and RFC 8785 preimage bytes (`NominalDeclaration`) on the
checked node. `UnitGraph` builds a `NominalUnitNode` for each admitted node,
with its terms, dimension and target resolved to admitted keys.
`UnitTable::declared` carries those nodes to lowering. There,
`Lowering::unit_nodes` adds each declared unit a quantity type names to the
graph, along with its dimension, base dimensions and target units. Each node
is keyed by its admitted bytes. The node keeps a typed nominal preimage only
when that preimage, rebuilt under the checked unit's `SourceOwner`, has
exactly the admitted bytes. The emitter writes the unit and dimension
preimages and their FR-093 rule-4 dependencies. Nodes another owner declares
are omitted as `UnlockedOwner`.

**The rebuild-and-compare approach is sound.** It cannot accept bytes that
differ from what check admitted:
- The node's `preimage` is always the admitted `NominalDeclaration` bytes
  and never the rebuilt ones. `admit_unit_graph` refuses any node whose key
  is not `retains(key, &preimage)`. Since `preimage_digest` and
  `preimage_bytes` serialize the same `canonical()` value, SHA-256 of the
  stored bytes is the key by construction.
- The rebuild only decides whether a typed preimage is attached, and only
  when its bytes are byte-equal to the admitted ones.
- The rebuild is exact for an own-owner node:
  - Admission refuses unreduced rationals, so the spelled scale and offset
    equal the canonical `Rational` that `UnitPreimage::new` respells.
  - Dimension terms are base-only (`NonBaseDimensionTerm`) and strictly
    ascending. The `BTreeMap<NodeKey, _>` they are rebuilt from has the same
    order.
- So a mismatch arises only for another owner.
- Downstream, IR's v2 reader (`validate_nominal_nodes`, quire-contract-ir
  ea63488) re-checks `preimage.digest() == node_id` and joins the owner
  against the lock.

**The TC-440 emit-and-IR-read oracle is real.** `emitted_measure` asserts
that nothing is omitted. The test asserts the unit node's tag and form, its
preimage version and qualified name, that its semantic type and its single
dependency are the dimension node, and the dimension's form and version.
Reaching `Lowered | RequiresBound` from IR proves that IR's reader admitted
the package, including the digest check on both preimages. The two-arm
disjunction is deliberate: it waits on IR-450, per the ruling.

**The FR-093-AC-19 and TC-160 step 8 rewording matches the code.**
- FR-093-AC-19: the admission corpus drops `KnownGap::UndeclaredUnit` and
  holds `dimension`, `unit` and `compound_unit` rows as admitted over a
  source-owned `metre`. Any omission now fails `check_fixture`.
- TC-160 step 8: `a_compound_unit_is_omitted_only_for_its_omitted_unit`
  asserts that the definition-owned `metre` and `Length` nodes are
  `UnlockedOwner`, that the compound unit is
  `NamesOmittedNode(metre)`, and the exact omitted set.
- The FR-062-AC-9 status, TC-416, TC-419, TC-440 and FR-097 text all match
  the tests.

Rust lane: there are no panics or `unwrap` in shipped code. The walk is a
heap work list with a visited check, so it terminates on the unit table's
finite node set. `quire-semantic-value` still builds `no_std`.

Gates at this head with `<worktree>/target`: tests for
`quire-semantic-value`, `qsl-semantics` and `qsl-package` pass, with the
ruled ignore. Clippy with `--all-targets -D warnings` is clean, and
`quire-semantic-value --no-default-features` builds.

## Verdict

Code is correct; one low cleanup.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `unit_nodes` matches `Owner::Model(_) => None`, but `Lowering::new` always sets `owner: Owner::Source(..)` and nothing reassigns it, so the arm is dead. Its implied meaning ("a model-owned lowering keeps no preimage") is behaviour the code never has. | qsl-semantics/src/check/lowering/model.rs:463-466 |

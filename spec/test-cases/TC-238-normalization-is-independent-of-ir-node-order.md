---
id: TC-238
title: "Replaying normalization over a permuted IR node order reproduces the same correspondence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-238: Replaying normalization over a permuted IR node order reproduces the same correspondence

## Description

Verify that normalizing the same set of declarations with their IR nodes
presented in two different orders produces byte-identical original keys,
effective identities and correspondence ordering. Scope: FR-081-AC-7.

Distinct from TC-216 (which varies a `displayName`/title field) and TC-217
(which varies the process the binder runs in): this test varies neither —
it varies only the order in which the same, otherwise byte-identical IR
nodes are presented to the binder, the axis quire-specification
FR-150-AC-4 names explicitly ("independent of IR node order").

Catches an implementation whose declaration-building or correspondence-sort
step is stable only by accident (for example, iterating a `HashMap` keyed
by declaration and relying on insertion order, or sorting only within one
phase while an earlier phase's output already carries presentation order)
— a defect invisible to any test that always presents nodes in one fixed
order.

Backed by the existing test
`tests/model_normalization.rs::n07_record_order_does_not_affect_identity_or_view`,
which normalizes `fixture_f2()` with its `records` reversed and asserts the
resulting view's own identity digest is byte-identical to the un-reversed
run's recorded vector. This directly exercises steps 1-4 and confirms the
combined set of original keys and effective identities is unchanged by
record order — a divergent key, identity or membership would change the
digest. It is a narrower guarantee than "correspondence ordering":
`n07` asserts one digest over the whole view, which pins the identity set
byte-for-byte, but does not separately assert that some per-entry iteration
sequence (as opposed to the view's own canonical, order-independent digest)
is unchanged. No existing test inspects a per-entry sequence directly.

## Test Procedure

1. Admit a domain package with at least four independent object-type
   declarations, several of them redefining or subsetting members of
   others, so the correspondence has real internal ordering to disturb.
2. Run the model binder's normalization over this domain package as
   admitted.
3. Build a second `DomainPackage` value whose declarations are the same
   set, byte-identical in content, but whose IR nodes are presented to
   intake in a different order (for example, reversed).
4. Run the model binder's normalization over the second domain package.

## Expected Results

The two normalization runs produce byte-identical original declaration
keys, byte-identical effective identities and byte-identical correspondence
ordering. A mutant whose correspondence ordering depends on IR node
presentation order (rather than sorting by declaration key or effective
identity) produces a different ordering between step 2 and step 4, failing
the byte-identical-ordering assertion even though every individual
key/identity value is still correct.

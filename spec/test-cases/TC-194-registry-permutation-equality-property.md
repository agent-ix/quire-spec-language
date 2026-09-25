---
id: TC-194
title: "Registry candidate sets are invariant under registration-order permutation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-290
    type: verifies
---
# TC-194: Registry candidate sets are invariant under registration-order permutation

## Description

Property-based test: for a fixed set of `BackendDescriptor` values, every
registry built by registering that set in any order is equal to every other
such registry, and computing candidate sets against each yields identical
sets in identical order for every item in a fixture item set. Scope:
FR-075-AC-2, FR-080-AC-1.

The fixed descriptor set is not required to be free of repeats or conflicts
(FR-075-AC-4, FR-075-AC-7; quire-specification FR-290-AC-9, FR-290-AC-10). A
multiset that includes an identical repeat of one identity's descriptor, and
a second identity with two conflicting descriptors, is built under every
sampled order; the resulting registries,
registration-refusal lists (`Registry::refusals`, in reported order) and
candidate sets must all agree, exactly as they do for a conflict-free set.
The registry's own definition (a `BTreeMap` of held descriptors plus a
`BTreeMap` of conflicted identities' recorded digests) is a function of the
admitted-registration set alone, so this is the same order-independence
property FR-075-AC-2 already states, applied to a multiset that exercises
FR-290's idempotent-repeat and conflict rules as well as its plain
one-registration-per-identity case.

This is the centerpiece non-vacuous test named by the ticket. Catches an
implementation that stores registrations in an insertion-ordered `Vec` and
returns matches in that order rather than sorting by `(identity, manifest
digest)`; an implementation using a `HashMap` whose iteration order is not
canonicalized before candidate-set construction; and an implementation that
happens to pass on one hand-picked pair of orderings (e.g. only testing
"forward" vs. "reversed") but fails on a third order the property generator
samples, which a fixed two-case example test would not catch.

[TC-155](TC-155-keep-admission-backend-independent.md) (FR-057-AC-8, on
`main`) already asserts order-independence as one step (forward order vs.
one reversed order) inside a broader fixture-based admission-and-routing
scenario. This test case is narrower and stronger on that one property: it
targets the registry (FR-075/FR-080) directly, not through admission, and
samples at least 20 generated orderings rather than one reversed pair, so
it is the test that can actually falsify an order-dependent registry
implementation that happens to pass on a simple forward/reverse check.

## Test Procedure

1. Construct a fixed set of at least three `BackendDescriptor` values with
   overlapping and non-overlapping advertised capability kinds.
2. Construct a fixed set of at least five requested items spanning kinds
   that zero, one, and more than one of the descriptors advertise.
3. Using a property-test framework's permutation generator (or an explicit
   enumeration when the descriptor count is small), build a registry for
   each of at least 20 distinct orderings of the descriptor set.
4. For each built registry, compute the candidate set for every item in the
   fixed item set from step 2, in the same order as returned by the
   registry.
5. Compare every ordering's registry (by content equality) and every
   ordering's per-item candidate-set sequence against the first ordering's
   registry and candidate-set sequences.
6. Separately, run the same procedure against a mutant registry
   implementation that appends registrations to a `Vec` and returns
   candidates in that `Vec`'s iteration order without sorting by
   `(identity, manifest digest)`.
7. Construct a second descriptor multiset that repeats one identity's
   descriptor identically and gives a second identity two conflicting
   descriptors (distinct manifest digests). Repeat steps 3-5 against it,
   additionally comparing every sampled ordering's registration-refusal list
   (in reported order) against the first ordering's.

## Expected Results

Steps 4-5: every permutation's registry is equal to every other
permutation's registry, and every permutation's per-item candidate-set
sequence is identical, in identical order, to every other permutation's
sequence, for all five items and all sampled orderings.

Step 6: the mutant implementation produces at least one item whose candidate
order differs between two sampled permutations, and the property test
reports a failure on that permutation, demonstrating the test is sensitive
to order-dependent candidate computation rather than vacuously passing.

Step 7: the repeat contributes no refusal and its identity is held once; the
conflicting identity contributes no candidate, is unregistered, and its two
refusals appear in every sampled ordering's refusal list, identically
ordered. The registry, its refusals and its candidate sets agree across
every sampled ordering, exactly as in steps 4-5.

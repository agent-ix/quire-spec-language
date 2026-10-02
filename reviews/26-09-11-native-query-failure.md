---
id: SR-346
title: "Failure-domain review of query iteration, guards, provenance and arithmetic bounds"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-040-check-composed-values.md; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-119-check-composed-values.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `spec-failure-domain-analysis` focused on query iteration, guard transfer,
original scope and provenance, and arithmetic bounds. The identity rules the
increment needed most — per-occurrence element identity and per-occurrence filter
guards — are explicit in the code and directly tested. The gaps are unstated
constraints: an arithmetic-cost bound nobody wrote down, an availability oracle
with two candidate authorities, and a normative non-simplification promise with no
control behind it. Rechecked at `baf93f5`: all three are closed, and the reworked
sum abstraction was re-derived from the code rather than accepted from its
comments.

## Verdict

**CONDITIONAL** (recheck) — all three mediums resolved at `baf93f5`; two low
unstated constraints remain (template nesting limit, mixed-denominator sum). No
missing failure mode that admits an unsound proof or an executable artifact.

## Findings

| ID      | Severity | Summary                                                                              | Refs                                                        | Escape Cause                    |
| ------- | -------- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------- |
| FND-001 | medium   | RESOLVED at `baf93f5` — no requirement bounds sum proof cost against declared capacity; exhaustion is the only outcome past it | spec/functional/FR-040-check-composed-values.md:143; src/checking/composed/proofs/engine/queries.rs:274 | missing-requirement             |
| FND-002 | medium   | RESOLVED at `baf93f5` — binder availability has two candidate authorities in the wire — scope handle and scope locus | docs/compiled-protocol-v1.md:533; src/protocol_artifact/native/layout.rs:462 | wrong-requirement               |
| FND-003 | medium   | RESOLVED at `baf93f5` — the normative "no witness, constant or unrolled graph" promise has no emission control for a statically empty query | docs/compiled-protocol-v1.md:535; tests/composed_query_proofs.rs:467 | correct-requirement-no-evidence |
| FND-004 | low      | Template reinstantiation depth is bounded only by the generic Depth high-water mark; no requirement states the nesting limit or its cause | src/checking/composed/proofs/engine/queries.rs:55; src/checking/composed/proofs/work.rs:72 | missing-requirement             |
| FND-005 | low      | A mixed-denominator sum (projection 1 into total 2) is reachable, refused as `SumDomainTransfer`, and still has no control | spec/functional/FR-040-check-composed-values.md:163; src/checking/composed/solver/validation.rs:163 | missing-requirement             |

### 1. Extension points and trust boundaries

The proof engine has one boundary that behaves like an extension point: a stored
`Sequence` template consumed by a later binder. Its failure behaviour is strict —
`element()` restores `locals` and `context` through a closure even on the error
path, so a failure mid-body cannot leak a reinstantiated environment into the
surrounding scope, and every failure becomes a typed `Cause` or a bounded
`Exhaustion`. Nothing is logged and suppressed. This is the right policy for a
proof gate and it is implemented; it is not stated anywhere as a requirement.

### 2. Entity identity

The uniqueness key for an iterated element is explicit in the code:
`Key::Element(binder, collection_key)` scoped by a read context that is freshly
minted on every template reinstantiation. Two traversals of the same source
collection under different binders therefore never identify their arbitrary
occurrences, and `IndependentElements` in `tests/composed_query_proofs.rs:667` is
a real control for it — `lhs != 0` does not discharge the divisor for `rhs`, and
the refusal lands at the original `rational(0,1) / rhs` span. `Division` is the
matching positive: within one occurrence, two mentions of the same binder are the
same symbol.

Filter selection facts follow the same rule. The guard is not attached to the
query value at creation; it is re-derived per consumption inside `element()`, so
it can guard a later map or query use only for that same occurrence. Both
directions are tested (`Guarded` third conjunct discharges, `PartialFilter`
refuses).

Captured locals keep their provenance across reinstantiation because `element()`
restores the template's `locals` snapshot rather than re-minting the capture, and
`Key::Binder`/`Key::Field` deliberately sit in context 0. The emitted contract
carries the matching evidence:
`nested_queries_preserve_body_scopes_and_captured_pre_origin_in_emitted_contracts`
asserts that reading a captured collection inside a query does not retag it as a
post observation.

None of this is written as a requirement. FR-040-AC-5 and AC-6 describe binder
shape and guard-before-use but do not state the occurrence-uniqueness key.

### 3. Evaluation purity

FR-040 and the wire contract are explicit that this phase does not execute
sequences: `size` and `count` overapproximate the declared range, quantifier body
totality is separated from the quantifier's eventual truth, and sum abstracts to
endpoint transitions. The code matches — the only place a concrete value appears
is the literal zero base and the declared domain endpoints, never a sample element
or runtime datum.

The one purity claim without a control is FND-003. The wire contract now promises
that "proof-side symbolic or empty-domain simplification does not replace the
emitted query with a witness, constant or unrolled graph". The proof side does
simplify: a contradictory filter drives `maximum` to 0, and `size`/`count`/`sum`
over it return a literal zero. The `Empty` predicate exercises exactly that path —
but only in `tests/composed_query_proofs.rs`, which never emits. No native
emission test feeds a statically empty query through `native::admit` and asserts
that the wire still carries a `Query { operator: Filter, .. }` rather than a
constant. The emitter takes its operations from the AST, not from the proof
report, so the promise is structurally true today; it is the kind of promise that
should have a control before someone routes proof results into lowering.

### 4. Topological robustness

Termination is guaranteed on the structures that exist. The `sequences` map is a
DAG by construction: each entry is keyed by a freshly minted `ValueKey` and refers
only to values built earlier in the traversal, so `element()` cannot cycle.
Worst-case nesting is bounded by the `Depth` dimension's high-water limit of 64,
charged before descent. Neither the acyclicity argument nor the nesting ceiling
appears in a requirement (FND-004), so a future change to template registration
has nothing to violate.

### 5. Arithmetic bounds

Sum checks start from literal zero and build actual shared-IR checked-addition
goals for both the lower and the upper endpoint transition at every prefix
`k <= N`, breaking only after the failing goal has been retained. Because addition
is monotone in both operands and the admitted domains are contiguous intervals,
the two endpoint transitions do enclose every k-occurrence prefix regardless of
order or duplicates, and no prior accumulator is assumed to inhabit the desired
Total. Accumulation widens to `i128` before the domain test and narrows through
`i64::try_from`, so the bookkeeping cannot itself overflow. Both the `N=6` whole-run
failure and the `Signed -10..10` intermediate-prefix failure are non-vacuous: in
the latter, prefix 1 passes and prefix 2 fails, and a later `accumulated -
accumulated = 0` rewrite cannot repair it.

What is unbounded is the *cost* (FND-001). The prefix loop is linear in the
declared capacity while every other query form is constant, and the goal ceiling
is hard-clamped downward for all callers, so past roughly half the declared
10,000 maximum the only reachable outcome is `Exhaustion`. That is a safe
outcome, but the spec neither states the bound nor says that exceeding it yields
exhaustion rather than a verdict.

The denominator gate is the other unstated arithmetic boundary (FND-005). Support
requires `maximum_denominator == 1` on both the projection and the total; a source
mixing them has no stated outcome and no control. Given the aggregate-domain type
checks that run first, the mixed case may be unreachable — which is itself worth
recording, because an unreachable arm in the sum domain check is currently
indistinguishable from an untested one.

### Recheck at `baf93f5`

**FND-001 resolved, and the failure domain of the sum abstraction changed.** The
per-prefix loop is gone; `sum_prefixes` now emits exactly two endpoint goals and
one `D::Expressions` charge per endpoint, so sum proof cost is constant in the
declared capacity and exhaustion is no longer reachable through capacity at all.
The arithmetic-bounds argument in §5 above still holds, with the coverage step
made explicit: because type admission requires `A <= 0 <= B` and `[a,b] ⊆ [A,B]`,
a prefix can only leave Total downwards through a negative `a` and upwards through
a positive `b`, and `k*a`/`k*b` are monotone, so the worst case for each direction
is `k = N`. `prefix = min(N, safe + 1)` picks the worst verified prefix or the
first crossing one, so a refusal still lands at the earliest failing `k` — checked
against `Prefix` (crossing at k=2 under N=10,000) as well as `LateUpper`/
`LateLower` (crossing first possible at k=10,000). FR-040's narrative now states
the interval and monotone-endpoint argument, so the transfer is a requirement
rather than a code comment, and AC-5 names caller-lowered exhaustion as distinct
from semantic refusal.

Two new failure paths were inspected for misreporting. The precondition block at
`queries.rs:303` mirrors `solver/validation.rs:152-164` exactly, so under an
admitted type it is unreachable and its `UpstreamType` refusal cannot be mistaken
for a semantic verdict; `TinyTotal 0..10` vs `Amount 1..20` is the tested case,
refusing upstream as `InvalidAggregateDomain` at the original sum with a
proof-side `UpstreamType`. `maximum == 0` is pre-empted by the zero-return at
`queries.rs:249`. The `i64::try_from` fallback on `prior` is likewise
unreachable, since `|prior| <= |bound|` by construction, and every widening
happens before the division and the multiplication, including `i64::MIN / -1`.

**FND-002 resolved.** `docs/compiled-protocol-v1.md:246,539` names the per-value
`scope` handle and the scope parent chain as the sole availability authority and
records that a scope locus keeps source provenance and may cover a collection
that cannot read the binder. One authority, one oracle.

**FND-003 resolved.** `statically_empty_filter_emits_its_original_collection_binder_and_body`
is the missing control: a `filter(kept in input.amounts: false)` whose proof-side
maximum is 0 still emits an original `Query { operator: Filter }` with its binder,
its `input.amounts` collection, a `Boolean { value: false }` body at the original
span and the retained sequence maximum 5, with `size`/`count`/`sum` still reading
the `absent` binder. The package survives `native::emit` and an independent read
with matching package and digest, so proof-only emptiness does not rewrite an
executable source graph.

**FND-005 sharpened.** The mixed case is no longer "may be unreachable" in one
direction: `solver/validation.rs:163` admits
`total.maximum_denominator() >= projection.maximum_denominator()`, so denominator
1 into denominator 2 reaches the proof engine and is refused as
`SumDomainTransfer`; the reverse mixing refuses upstream as
`InvalidAggregateDomain`. Only the both-denominator-2 case has a test. FND-004 is
unchanged.

### Not established by these controls

Query runtime execution, B consumer conformance, recovery, general protocol
decisions, and full FR-040/FR-042 acceptance remain outside what this increment
demonstrates. An empty producer domain stays refused at the already adopted model
boundary; the runtime failure domain for an admitted max-positive collection that
is empty at execution is not addressed here and belongs to the runtime validation
requirement.

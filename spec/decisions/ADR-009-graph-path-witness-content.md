---
id: ADR-009
title: "What a graph path witness carries"
type: ADR
status: proposed
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-047
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: depends_on
---
# ADR-009: What a graph path witness carries

## Status

**Open question for the architect.** [FR-047](../functional/FR-047-evaluate-finite-object-reference-graphs.md)
currently returns a Boolean reachability result with no path witness, by
explicit scoping choice. This decision does not change that scope. It records
the open question so that if and when a caller-visible path witness is
admitted, its content is fixed once rather than re-derived per consumer.

## Context

[FR-047](../functional/FR-047-evaluate-finite-object-reference-graphs.md)'s
Outputs section states reachability returns no path witness, and its Behavior
section places canonical path witnesses outside this requirement's scope. The
evaluator already performs deterministic depth-first search in supplied
edge-occurrence order, expands each complete storage identity
(`observation occurrence, model, universe, object type, object identifier`)
at most once per request, and charges expansion, edge and comparison work
under [NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md)'s
`quire.state.evaluation-work/1` accounting.

A future consumer of a `true` reachability result may need to know which path
the evaluator found, not only that one exists — for diagnostics, replay, or
downstream native emission. Admitting a path witness changes the evaluator's
output type and interacts with three settled behaviors that any witness
content must stay consistent with: deterministic first-found order (not
shortest path), at-most-once expansion per storage identity, and
charge-before-work accounting. FR-047 excludes a witness rather than defining
one, and one constraint on witness content already exists upstream: the
vendored language standard at the pinned baseline,
`resources/native-v1/proposals/quire-v1/state-contract.md`, states that any
returned path witness retains the actual typed edge and snapshot identities,
and that the Boolean reachability predicate adds no shortest-path or
canonical-witness guarantee. Any answer has to sit inside that sentence, and
the candidates below differ in how they do so.

## Question

What does a graph path witness carry, when some future requirement admits
one?

Candidate answers:

1. **Ordered edge sequence.** The witness is the ordered list of resolved
   edges (source complete storage identity, edge field, target complete
   storage identity) traversed from the reachability start to the target,
   in the same deterministic depth-first order the evaluator already uses to
   decide `true`.
2. **Ordered node sequence.** The witness is the ordered list of complete
   storage identities visited from start to target, omitting which edge
   field connected each pair.
3. **Opaque replay token.** The witness carries enough retained provenance
   (the admitted package, immutable graph input and the evaluator's
   deterministic search order) to let a caller re-run the same deterministic
   search and recover the same path, without the evaluator itself emitting
   path content.
4. **No witness is ever admitted.** Reachability stays permanently
   Boolean-only; a caller who needs a path must issue a separate, explicitly
   scoped traversal request outside FR-047 (consistent with FR-047's current
   exclusion of mutable graph execution and shortest-path selection).

Against the standard's sentence quoted in Context, candidate 1 carries the
typed edge and snapshot identities it names directly, candidate 2 carries the
snapshot identities without the edge identity, candidate 3 supplies both only
after a caller re-runs the search, and candidate 4 leaves the sentence with no
witness to govern. Which of these the owner selects is the open question.

Answering this fixes: whether a path witness is a new evaluator output type
or a derived replay artifact, whether its content must remain stable under
retry with fresh work accounting, and whether it requires its own
[NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md) charge
dimension distinct from reachability's existing charges.

## Consequences

While this question is open, FR-047's Boolean-only reachability and its
explicit exclusion of canonical path witnesses remain the current design;
this decision holds the question rather than settling it, and no code or
spec change here changes FR-047's current output. A downstream requirement
that wants a path witness cites this decision rather than inventing witness
content locally.

---
id: FR-047
title: "Evaluate finite typed object-reference graphs"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-038, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-039, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-043, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-045, type: depends_on }
---
# FR-047: Evaluate finite typed object-reference graphs

## Description

When the selected graph profile supplies a complete finite typed environment,
the language pipeline SHALL preserve and evaluate exact object identity,
dereference and positive-length reachability without ambient lookup.

This requirement retrospectively scopes roadmap L4 under compiler
[#37](https://github.com/agent-ix/quire-spec-language/issues/37). It adopts the
finite-graph extension in the immutable composed-v1 baseline without changing
the historical ConfigVersion profile or inventing finite bounds for that model.

## Inputs

An admitted compiled value graph; exact model, universe, object-type, reference
field and observation selections; immutable current, invocation-pre or
invocation-post anchors; a finite object arena keyed by the complete storage
identity `(observation occurrence, model, universe, object type, object
identifier)` and preserving reference occurrence order; explicit membership
and closure authority; and caller-lowered finite evaluation limits admitted by
[FR-049](FR-049-admit-composed-evaluation-inputs.md).

The observation occurrence is the exact semantic anchor, snapshot identity,
optional window identity and record identity supplied by F under the selected D
assessment authority. Those identities remain opaque producer/observation
identities; they are not compiled-protocol `ArtifactRef` values and do not reuse
the compiled artifact's raw-byte digest domain.

An object identifier string, payload field, trace identifier, same-shaped model
or successful static type check is not a graph environment. D/F remain the
authorities for concrete object, relationship, population, snapshot and closure
inputs.

## Outputs

Exact typed identity or Boolean results with the selected observation, or a
typed incomplete, refused or resource-exhausted outcome with deterministic work
usage. Reachability returns no path witness. No failed traversal exposes a
partial Boolean or substitutes `false` for inability to decide.

## Behavior

The compiler SHALL emit graph operations with their original value handles,
model/universe/object type, resolved edge field, source locus and observation
anchor.

The graph input validator SHALL reject duplicate complete storage identities.
It SHALL permit the same model/universe/object-type/object-identifier tuple
under distinct pre, post or current anchors, snapshots, windows or record
occurrences as separate storage entries.

If the selected membership and closure authorities declare the snapshot domain
complete, then the graph input validator SHALL refuse a reference whose exact
full-storage-key target is absent from that domain.

If required membership or closure authority is unavailable, then the graph
input validator SHALL return incomplete input without interpreting the supplied
subset as closed or classifying an absent offered target as dangling. A known
foreign model, universe, object type, reference field or observation remains a
typed refusal even when completeness is unavailable.

The evaluator SHALL resolve `deref` only through the reference's exact selected
model, universe, object type, identity and complete observation occurrence.

The evaluator SHALL compare object/reference identity by exact model, universe,
stable object type and object identifier. It SHALL project away only the
complete observation occurrence for an explicitly admitted cross-observation
identity comparison; field reads, dereference and traversal retain the complete
storage identity.

The evaluator SHALL require both endpoints of `reaches(a,b,edge)` to share the
selected model, universe, object type and snapshot.

The evaluator SHALL accept only an edge resolved on that type as `Ref(T)`,
`Option(Ref(T))` or ordered `Seq(Ref(T),N)` for this graph profile.

The evaluator SHALL interpret reachability as a path of one or more edges.

The evaluator SHALL perform deterministic depth-first search in supplied edge-
occurrence order and test each discovered endpoint before suppressing repeated
expansion.

The evaluator SHALL expand each complete object storage identity at most once
per reachability request while preserving and charging repeated edge
occurrences as input.

The evaluator SHALL return true for a self-loop or cycle that reaches its start
again. It SHALL return false only after deterministic traversal finds no target
in the selected declared-complete snapshot domain; an isolated start in such a
domain is false.

The evaluator SHALL use `quire.state.evaluation-work/1` and the charge-before-
work depth-first-search rules in
[NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md). The selected
start has active graph depth one; each recursively entered unexpanded storage
key adds one active frame. If the next expansion, edge, comparison or active-
depth charge exceeds the clamped ceiling, then evaluation SHALL return typed
resource exhaustion without a Boolean result.

The evaluator SHALL start a retry with fresh work accounting over the unchanged
admitted package and immutable graph input.

The pipeline SHALL preserve historical definitions and the original
ConfigVersion source/model identities without adding a guessed integer maximum,
parent relationship or graph authority.

Mutable graph execution, shortest-path selection, canonical path witnesses,
untyped heterogeneous traversal, network lookup, scalar-ID navigation and graph
repair remain outside this scope. Protocol relationships remain first-class
producer-owned bindings; they do not become fields merely to reuse `reaches`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-047-AC-1 | Exact anchor, snapshot, optional window, record occurrence, model, universe, object type, object identifier and edge type survive parse, binding, checking, admission, emission and independent reading as the complete storage identity; equal strings or shapes cannot substitute and producer/observation identities do not become compiled-artifact references. | Test (TC-129) |
| FR-047-AC-2 | A duplicate full storage key and a dangling target in a declared-complete domain refuse, while the same logical object at distinct pre/post anchors is admitted. Foreign models/universes, wrong object/reference types, wrong edge owners and incompatible endpoint observations refuse; missing membership or closure is incomplete and cannot turn an absent offered target into a dangling or empty-graph judgment. | Test (TC-129, TC-131) |
| FR-047-AC-3 | Dereference reads only the exact qualified environment and retains pre/post observation identity across the returned object. | Test (TC-129) |
| FR-047-AC-4 | Reachability is positive-length: an isolated `a` does not reach itself, while `a→a` and `a→b→a` do; a direct or longer path to another target succeeds. | Test (TC-130) |
| FR-047-AC-5 | Deterministic depth-first search expands each full storage identity at most once, visits sibling edges in authored occurrence order and charges duplicate edges independently without changing Boolean reachability. | Test (TC-130) |
| FR-047-AC-6 | Optional absence and empty edge sequences have no outgoing edge; only `Ref(T)`, `Option(Ref(T))` and `Seq(Ref(T),N)` edges are admitted by this profile. | Test (TC-130) |
| FR-047-AC-7 | Zero, exact, one-step-insufficient and above-hard requests under `quire.state.evaluation-work/1` distinguish completed true/false results from typed resource exhaustion; a sufficient retry starts fresh and returns the same Boolean with deterministic usage. | Test (TC-131) |
| FR-047-AC-8 | Historical ConfigVersion parsing, linkage and evaluation retain their original identities and parent semantics without a fabricated finite numeric or graph selection. | Test (TC-131) |

## Dependencies

[FR-036](FR-036-link-composed-native-packages.md) owns exact namespace and model
binding and [FR-040](FR-040-check-composed-values.md) owns static
type/definedness admission. The already implemented artifact-core portions of
[FR-042](FR-042-publish-compiled-protocol-artifacts.md)—schema, strict reader,
bounded admitted package and emitted value graph—supply the immutable graph;
FR-042's family-completeness and consumer-handoff criteria do not precede graph
evaluation. [FR-049](FR-049-admit-composed-evaluation-inputs.md) owns
the immutable storage/input admission boundary and
[NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md) owns the
versioned work identity, hard ceilings and charge order. The immutable standard's
FR-043 supplies the selected graph meaning. Concrete object membership and
closure are independently supplied runtime inputs. This retrospective cycle under
[#66](https://github.com/agent-ix/quire-spec-language/issues/66) records that
earlier graph code and tests preceded this compiler-owned requirement.

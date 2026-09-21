# Native finite graph definition

Profile identity: `quire.state.graph/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

This is a working definition in one coherent draft. Required definitions and
rule links resolve within this checkout; acceptance freezes their exact selections.
Earlier draft text remains in Git history. Runtime references still require the
selected immutable identities, revisions and digest domains.

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `quire.state.queries/v1` | `1-draft.3` | [Predicates and queries](state-queries.md) |

## Explicit extension

Retain the complete core and query contracts. Add identity-bearing object and
reference values, explicit dereference through their typed finite supplied
universe, object/reference identity equality and positive-length `reaches`.
Identity includes universe, declared object type and object identifier; source
sequences retain repeated reference occurrences. Reachability checks the target
before suppressing repeated expansion and expands each identity at most once.
A self-loop can establish positive-length self-reachability.

Snapshot-qualified references retain the observation they read. An identity
comparison across pre/post does not permit a field read to switch observations.
Missing targets, foreign universes, unknown closure and resource exhaustion
cannot become false reachability or empty populations. The exact model and
population bindings supply this authority; no scalar-to-object cast, implicit
foreign-key lookup, recursive record expansion or ambient store is introduced.

This is the state profile providing the complete required v1 state/query/graph
scope. It does not supply temporal/protocol semantics, realize their requested
backend claims, or qualify the original minimum-only ConfigVersion model.
That original model still needs an explicit authored finite-domain disposition
and exact relationship mapping; its historical example is preserved.

## Selected graph rules

Core/query definitions and their current state contract remain inherited
dependencies. The following current rule supplies the graph extension.

| Rule |
| --- |
| [FR-043](../../../spec/functional/FR-043-evaluate-finite-graph-relations.md) |

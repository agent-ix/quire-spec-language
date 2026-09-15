# Native predicate and sequence-query definition

Profile identity: `quire.state.queries/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

This is a working definition in one coherent draft. Required definitions and
rule links resolve within this checkout; acceptance freezes their exact selections.
Earlier draft text remains in Git history. Runtime references still require the
selected immutable identities, revisions and digest domains.

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `quire.state.core/v1` | `1-draft.3` | [State core](state-core.md) |

## Explicit extension

Retain all state-core forms and constraints. Add typed acyclic named Boolean
predicate declarations/calls, `filter`, `map`, `count` and `sum`.
Queries preserve occurrence order and duplicates; sum performs the explicit
same-representation/same-unit transfer into its named result domain, with exact
zero and every-prefix bounds. No implicit flattening, general casts or unit
conversion is introduced.

Predicate parameters and captures are explicit immutable typed inputs.
A reusable predicate has no ambient self, operation result or historical state.
An imported callee keeps its checked profile, type and anchor interpretation.
General value-returning functions, recursive calls, dynamic dispatch, overloads
and hidden effects remain refused. Reference/object identity, navigation and
reachability remain refused until the graph extension is selected.

Other families may consume this profile's checked Boolean predicate and shared
value-expression interface with explicit argument/anchor bindings. That interface
does not itself grant temporal/protocol declaration admission or turn a monitor
status into a Boolean. A caller selecting this profile cannot upgrade a callee
bound to state core.

## Selected extension rules

State-core rules remain inherited dependencies. The following current rules
supply the explicit query and predicate extension.

| Rule |
| --- |
| [FR-033](../../../spec/functional/FR-033-admit-reusable-predicates.md) |
| [FR-034](../../../spec/functional/FR-034-bind-cross-family-predicates.md) |
| [FR-041](../../../spec/functional/FR-041-evaluate-ordered-queries.md) |
| [FR-042](../../../spec/functional/FR-042-select-pre-state-reads.md) |

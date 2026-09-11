# Native finite state core definition

Profile identity: `quire.state.core/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

This is a working definition in one coherent draft. Required definitions and
rule links resolve within this checkout; acceptance freezes their exact selections.
Earlier draft text remains in Git history. Runtime references still require the
selected immutable identities, revisions and digest domains.

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `ix:native` / `1-draft` | `1-draft.2` | [Edition](edition.md) |

## Admitted forms and meaning

This profile admits invariant, operation precondition and postcondition
declarations. Shared expressions admit Boolean, contextually typed bounded
integer, explicit rational, text and enum literals; resolved value/parameter
reads; acyclic record paths; grouping, `let`, `if`; `not`, `and`,
`or`, `implies`; admitted scalar equality/order; numeric negate, `+`, `-`,
`*` and rational `/`; `present`, guarded `value`, `size`, `contains`,
`forall`, `exists`; and invocation-bound `pre(expr)` in postconditions.

All source constructs and retained model declarations must satisfy the selected
state rules, including unused declarations and unselected branches. Range and
nonzero proofs apply to potentially evaluated operations under sound native
control facts. Scalar types, units, bounds, presence and source declarations
come from exact supplied model contracts. No default numeric maximum or field
is derived from example data. Sequences are finite, ordered and duplicate
preserving with authored maxima at most 10,000 per wrapper.

Whole-record/sequence/option equality, reference/object identity and navigation,
`reaches`, named predicate declarations/calls, `filter`, `map`, `count`
and `sum` require other definitions and are refused by this profile.
No sequence indexing or slicing production is selected by the shared grammar.
Integer slash/div/rem/mod, float, implicit conversion, silent widening,
recursive value containment, ambient lookup, reflection and I/O are refused.
Rational division uses the same named dimensionless rational type, exact
nonzero proof and normalized result bounds; it never admits integer division.

Temporal/protocol declarations require their own profiles. A state-core success
does not claim full composed-v1 capability. Historical `state-finite/0-draft`
and the first-profile amendment retain their separate identities; this new
edition/profile does not relabel their sources or results.

## Selected state rules

The state contract defines semantics for admitted forms; descriptions of query
or graph extensions do not grant those forms here. Its earlier first-profile
comparison is compatibility context; the exact admitted-form catalog above and
selected composed rules define this candidate.

The following current rules supply the admitted state meaning.

| Rule file |
| --- |
| [state-contract.md](../state-contract.md) |
| [FR-039-normalize-exact-rational-literals.md](../../../spec/functional/FR-039-normalize-exact-rational-literals.md) |
| [FR-044-check-composed-numeric-definedness.md](../../../spec/functional/FR-044-check-composed-numeric-definedness.md) |
| [FR-045-preserve-control-and-presence-facts.md](../../../spec/functional/FR-045-preserve-control-and-presence-facts.md) |
| [FR-046-validate-state-invocation-inputs.md](../../../spec/functional/FR-046-validate-state-invocation-inputs.md) |

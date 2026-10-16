---
id: TC-1046
title: "reaches over a relationship end types as Boolean and refuses by name, operand, direction and destination"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-365
    type: verifies
---
# TC-1046: reaches over a relationship end types as Boolean and refuses by name, operand, direction and destination

## Description

Verify that the `StateModel` hook checks `reaches(a, b, name)` when `name` is a
relationship end, and refuses each inadmissible form with its code and cause.
Scope: FR-365-AC-1 to FR-365-AC-4.

Catches a checker that resolves the third argument only as a field (every
positive vector then refuses `missing-name`), that skips the direction or the
destination checks, that reports one cause for every refusal, or that breaks
the field-edge form.

## Test Procedure

Compile one unit per vector over fixture **R** with model alias `M` and profile
alias `V`, writing the expression in the clause shown. `self` is the context
object and `target` is the parameter of `M::Order::reroute(target:
Reference<M::Order>)` in a `pre` or `post` clause. Compare the checked node's
static type or the refusal's code, cause, family and location with the vector.

Fixture **R** (domain package `test/orders`) declares object types `Order`,
`Rush` (supertype `Order`) and `Person`; `Order` declares the field `via` typed
`Option<Reference<Order>>` and the field `parent` typed
`Option<Reference<Order>>`; a population `orders` with member type `Order` and no
maximum, and a population `people` with member type `Person`. Each relationship
is declared by its source type; the end names type, role and multiplicity
`{lower,upper}`, with `unique: true` and `ordered: false` unless stated:

| Relationship | Direction | Source end | Target end |
| --- | --- | --- | --- |
| `Order/next` | `source-to-target` | `Order`, role `next`, `{0,1}` | `Order`, role `previous`, `{0,1}` |
| `Order/link` | `bidirectional` | `Order`, role `outgoing`, `{0,1}` | `Order`, role `incoming`, `{0,1}` |
| `Order/mate` | `undirected` | `Order`, role `mate`, `{0,1}` | `Order`, role `mateOf`, `{0,1}` |
| `Order/back` | `target-to-source` | `Order`, role `back`, `{0,unbounded}` | `Order`, role `backOf`, `{0,unbounded}` |
| `Order/kids` | `source-to-target` | `Order`, role `kids`, `{0,1}` | `Order`, no role, `{0,3}` |
| `Order/crowd` | `source-to-target` | `Order`, role `crowd`, `{0,1}` | `Order`, no role, `{0,unbounded}` |
| `Order/ranked` | `source-to-target` | `Order`, role `ranked`, `{0,1}` | `Order`, no role, `{0,1}`, `ordered: true` |
| `Order/bare` | `source-to-target` | `Order`, role `bare`, `{0,1}` | `Order`, no role, no multiplicity |
| `Order/owner` | `source-to-target` | `Order`, role `owner`, `{0,1}` | `Person`, no role, `{0,1}` |
| `Order/parent` | `source-to-target` | `Order`, role `parent`, `{0,1}` | `Order`, no role, `{0,1}` |
| `Order/twinA` | `source-to-target` | `Order`, role `twin`, `{0,1}` | `Order`, no role, `{0,1}` |
| `Order/twinB` | `source-to-target` | `Order`, role `twin`, `{0,1}` | `Order`, no role, `{0,1}` |
| `Order/loose` | `source-to-target` | `Order`, role `loose`, `{0,1}` | `Person`, no role, `{0,unbounded}` |

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| C01 | `not reaches(self, self, next)` as an `invariant` on `M::Order` | admitted, type `Boolean`; the node holds relationship `Order/next` and role `next` |
| C02 | `reaches(self, target, next)` as a `pre` clause, and as a `post` clause, of `reroute` | admitted, `Boolean` each |
| C03 | `reaches(self, target, next)` as an invariant on `M::Rush` with `target: Reference<M::Order>`, and with `target` a `Reference<M::Rush>` | admitted: each operand conforms to the near end's type `Order` |
| C04 | `reaches(self, self, incoming)` on `M::Order` (`bidirectional`); `reaches(self, self, mateOf)` (`undirected`); `reaches(self, self, outgoing)` | admitted, `Boolean` each |
| C05 | `reaches(self, self, via)` on `M::Order` | admitted: the field-edge form checks as FR-104 states |
| C06 | `reaches(self, self, parent)` on `M::Order` (the field `parent` and the end `parent`); `reaches(self, self, twin)` (two ends) | `ambiguous_declaration`/`ambiguous-name` at the third argument, naming both candidates, each |
| C07 | `reaches(self, self, nothing)` | `missing_declaration`/`missing-name` at the third argument |
| C08 | `reaches(self, target, next)` with `target: Reference<M::Person>`; and with `target: Int[0,9]` | `ill_typed`/`operator-ineligible` at the form, each |
| C09 | `reaches(self, self, previous)` on `M::Order` (the inverse of `source-to-target`) | `ill_typed`/`operator-ineligible`: direction |
| C10 | `reaches(self, self, kids)` (finite multi-valued destination); `reaches(self, self, owner)` (destination `Person`) | `ill_typed`/`operator-ineligible`, each |
| C11 | `reaches(self, self, crowd)`; `reaches(self, self, ranked)`; `reaches(self, self, bare)` | `unsupported_construct`/`expression-form`, each |
| C12 | `reaches(self, self, back)` on `M::Order`: forward traversal of a `target-to-source` relationship whose destination is also unbounded | `ill_typed`/`operator-ineligible` (direction), not `expression-form`: direction before multiplicity |
| C13 | `reaches(self, self, next)` in a `Value` function body `f(o: Reference<M::Order>): Boolean pure` | `ill_typed`/`operator-ineligible` at the form: no observation to traverse |
| C14 | `reaches(self, self, M::Order::next)` | refused at the form's build as FR-102 states for a qualified edge |
| C15 | `Person` declares the operation `watch(o: Reference<M::Order>)`; the clause `pre using V on M::Person::watch { reaches(o, o, next) }` (`next` resolves on `Order`, the operand's type, as the end of `Order/next`; the context object is a `Person`, so the walked population is not the context population); then with an added population `archive` with member type `Order` and no maximum | admitted, type `Boolean`, and the requirements record adds the `orders` domain once beside the context population `people`; then `ambiguous_declaration`/`ambiguous-name` at the form naming `archive` and `orders` in ascending `DeclarationKey` order |
| C16 | `reaches(self, self, loose)` on `M::Order`, where `Order/loose`'s destination is an unbounded `Person` end (it fails both the multiplicity and the destination-type check) | `unsupported_construct`/`expression-form`, not `ill_typed`/`operator-ineligible`: multiplicity before destination type |

## Expected Results

Each refusal is the one named for its condition alone, with `StateModel`
family and the catalog code carrying the `state-model` prefix. A relationship
end that passes the five checks types `Boolean`; the field-edge form is
unchanged.

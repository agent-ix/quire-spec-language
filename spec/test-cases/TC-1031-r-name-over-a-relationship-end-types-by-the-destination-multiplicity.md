---
id: TC-1031
title: "r.name over a relationship end types by the destination multiplicity and refuses by name, direction and bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-363
    type: verifies
---
# TC-1031: r.name over a relationship end types by the destination multiplicity and refuses by name, direction and bound

## Description

Verify that the `StateModel` hook types `r.name` over a relationship end and
refuses each inadmissible navigation with its cause. Scope: FR-363-AC-1 to
FR-363-AC-5.

Catches a checker that treats every `r.name` as a field read (an end then
refuses `ill_typed`/`type-mismatch`, so every positive vector fails), that
ignores direction or the destination bound, or that reports one cause for
every refusal.

## Test Procedure

Compile one unit per vector over fixture **N** with model alias `M` and
profile alias `V`, writing the expression in
`invariant I using V on M::<owner> at current { ... }` with `self` the
receiver, except N13 and N14, which are function declarations as written.
Compare the checked node's static type, or the refusal's code, cause and
family, with the vector.

Fixture **N** (domain package `test/orders`) declares object types `Order`,
`Fulfilment`, `Rush` (supertype `Fulfilment`), `Line`, `Tag`, `Person`, `Step`,
`Note`, `Carrier`, `Dual` and `Rival`. Each relationship is declared by its
source type; the end names type, role and multiplicity `{lower,upper}`, with
`unique: true` and `ordered: false` unless stated:

| Relationship | Direction | Source end | Target end |
| --- | --- | --- | --- |
| `Fulfilment/order` | `source-to-target` | `Fulfilment`, role `order`, `{0,unbounded}` | `Order`, no role, `{0,1}` |
| `Fulfilment/carrier` | `source-to-target` | `Fulfilment`, role `carrier`, `{0,unbounded}` | `Carrier`, no role, `{1,1}` |
| `Order/lines` | `bidirectional` | `Order`, role `lines`, `{1,1}` | `Line`, role `owner`, `{0,3}` |
| `Order/tags` | `source-to-target` | `Order`, role `tags`, `{0,unbounded}` | `Tag`, no role, `{1,2}`, `unique: false` |
| `Order/watchers` | `source-to-target` | `Order`, role `watchers`, `{0,unbounded}` | `Person`, no role, `{0,unbounded}` |
| `Order/steps` | `source-to-target` | `Order`, role `steps`, `{0,unbounded}` | `Step`, no role, `{0,3}`, `ordered: true` |
| `Order/notes` | `source-to-target` | `Order`, role `notes`, `{0,unbounded}` | `Note`, no role, no multiplicity |
| `Order/twin` | `bidirectional` | `Order`, role `twin`, `{0,1}` | `Order`, role `twin`, `{0,1}` |
| `Order/rival` | `target-to-source` | `Order`, role `rival`, `{0,1}` | `Rival`, role `rivalOf`, `{0,1}` |
| `Order/peer` | `undirected` | `Order`, role `peer`, `{0,1}` | `Person`, role `peerOf`, `{0,1}` |
| `Dual/link` | `source-to-target` | `Dual`, role `link`, `{0,unbounded}` | `Order`, no role, `{1,1}` |

`Dual` also declares the field `link` typed `Order` `{1,1}`. `Order` declares
the field `code` typed `Int[0,9]`.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| N01 | `present(self.order)` on `M::Fulfilment` | admitted; `self.order` has type `Option<Reference<M::Order>>` |
| N02 | `self.carrier` on `M::Fulfilment` | `Reference<M::Carrier>` |
| N03 | `self.order` on `M::Rush` | `Option<Reference<M::Order>>`: the receiver conforms to the source end's type through its supertype |
| N04 | `self.lines` on `M::Order`; `self.owner` on `M::Line` | `Set<Reference<M::Line>>[0,3]`; `Reference<M::Order>`: the target end's role names the inverse end under `bidirectional`, whose destination is the source end `{1,1}` |
| N05 | `self.tags` on `M::Order` | `Bag<Reference<M::Tag>>[1,2]` |
| N06 | `self.peer` on `M::Order`; `self.peerOf` on `M::Person` | `Option<Reference<M::Person>>`; `Option<Reference<M::Order>>`: `undirected` admits both traversals |
| N07 | `self.twin` on `M::Order`; `Dual` read as `self.link` | `refused { code: ambiguous_declaration, cause: ambiguous-name }` naming the source and the target end of `Order/twin`; the same refusal naming the field `Dual/link` and the end `Dual/link` |
| N08 | `self.nope` on `M::Order` | `refused { code: ill_typed, cause: type-mismatch }` |
| N09 | `self.rival` on `M::Order`; `self.rivalOf` on `M::Rival`; `self.owner` on `M::Line` with `Order/lines` changed to `source-to-target` | `refused { code: ill_typed, cause: operator-ineligible }` (forward against `target-to-source`); `Option<Reference<M::Order>>`; `refused { code: ill_typed, cause: operator-ineligible }` (inverse against `source-to-target`) |
| N10 | `self.watchers` on `M::Order`; `self.steps` on `M::Order`; `self.notes` on `M::Order` | each `refused { code: unsupported_construct, cause: expression-form }` |
| N11 | `Order/watchers` changed to direction `target-to-source`, then `self.watchers` on `M::Order` | `refused { code: ill_typed, cause: operator-ineligible }`: the direction refusal precedes the unbounded-bound refusal |
| N12 | `deref(self).order` on `M::Fulfilment`; `self.order.order` on `M::Fulfilment`, whose receiver `self.order` is an `Option<Reference<M::Order>>` | `refused { code: ill_typed, cause: type-mismatch }` at the form; `refused { code: ill_typed, cause: type-mismatch }` at the receiver operand `self.order` |
| N13 | `function f using V(o: Option<Reference<M::Order>>): Boolean pure { o.lines = o.lines }`; the same with `o: Set<Reference<M::Order>>[0, 2]` | each `refused { code: ill_typed, cause: type-mismatch }` at the receiver operand `o`, as in an invariant |
| N14 | `function f using V(o: Reference<M::Order>): Boolean pure { size(o.lines) >= 0 }` | admitted: a `Reference` receiver navigates in a function body |

Each refusal's family is `StateModel` and its catalog code carries the
`state-model` prefix (FR-300).

## Expected Results

Every vector gives exactly its type or refusal. A mutant that removes the
relationship-end typing fails N01 to N06; a mutant that skips the direction
check fails N09 and N11; a mutant that skips the bound check fails N10; a mutant that types a non-`Reference` receiver by context fails
N13.

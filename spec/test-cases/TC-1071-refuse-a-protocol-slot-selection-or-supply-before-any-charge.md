---
id: TC-1071
title: "Refuse a protocol slot selection or supply before any charge"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1071: Refuse a protocol slot selection or supply before any charge

## Description

Verify that S6a refuses a slot that names no expression, a supply that is not
exactly the slot's sources, and a value that does not fit its binder, each with
one record and with nothing charged. Scope: FR-390-AC-3 to FR-390-AC-6.

Catches a crossed node or member that evaluates a neighbour, a missing source
that evaluates as false or zero, a supplied value the expression does not read
that is ignored, another template's binder read through a shared name, and a
wrong, out-of-domain, foreign or dangling value that evaluates.

Fixture **E** and the observation of TC-1070. The base call is TC-1070 row 8:
the slot `Refund.retry.relation` with the supply `earlier = Attempt(tally 0)`,
`later = Attempt(tally 1)`, `target = 5`, `activated = false`. Every row below
changes the base call as it states, asserts the one refusal's code, cause and
locus, that the evaluator returned no `Evaluation`, and that the meter's
counters are as they were before the call.

Reference-bearing supplies below use FR-106's `ProtocolInputValue` with a
`ReferencePopulation` descriptor for every occurrence. A root reference has the
empty shared path; other values with no references use an empty descriptor
list. The selected population's exact checked node key is explicit even for a
ghost reference or a call with no observation. Plain values in the table are
notation for the wrapper, not an implicit scope conversion.

### Nested population fixture

For rows 23 onward, keep E's source unchanged and extend its admitted model:
`Receipt` has `peer: Reference<Other>`, `Other` has `total: Total`, and
`otherNodes` and `otherSpare` each declare member `Other`. `Other` is not a
direct protocol context, so this package checks; declaring a direct context
whose binding is ambiguous is not this fixture. The two populations share the
reference's universe and effective object type. Select
`Refund.registration_captures[0]`, supply `forward = Receipt(total 5, peer o1)`
with the descriptor path `[Field("peer")]`, and use a successful current
observation authority. Initially `otherNodes` is complete and holds `o1`, and
`otherSpare` is unlisted. The initializer reads `forward.total`, but admission
still checks the nested peer. Its kernel reference and supplied record remain
unchanged when only the explicit population descriptor is changed.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | the node key of no node of the package | `missing_declaration`/`missing-name` at the node |
| 2 | the node key of a `state_clause` node of the package | `invalid_model_binding`/`malformed-declaration` at the node |
| 3 | the node `Refunds` (the clause) with member `retry.relation` | `missing_declaration`/`missing-name`, naming `retry.relation` |
| 4 | the node `Refund` with member `registration_captures` and index 1 (the template holds one) | `missing-name`, naming `registration_captures` and index 1 |
| 5 | the node `Refund` with member `registration_captures` and no index | `missing-name`, naming `registration_captures` and the missing index |
| 6 | the node `Refund` with member `retry.relation` and index 0 | `missing-name`, naming `retry.relation` and index 0 |
| 7 | a protocol that declares `terminal any`, the slot `terminal.condition` of that obligation | `missing-name`, naming `condition` (mode `any` holds none) |
| 8 | `earlier` supplied twice | `invalid_runtime_input`/`duplicate-member`, naming `earlier` |
| 9 | the supply adds `forward = Receipt(total 5)` | `invalid_runtime_input`/`unknown-member`, naming `forward` (the retry relation does not read it) |
| 10 | the supply adds `partialTarget = 5` | `unknown-member`, naming `partialTarget` |
| 11 | the slot `Refund.trigger.guard` with `target = 5`, `partialTrigger = Flag(ready false)` | `unknown-member`, naming `partialTrigger`, though `trigger` is also missing (unknown before missing) |
| 12 | the supply omits `activated` | `invalid_runtime_input`/`missing-member`, naming `activated` |
| 13 | the supply omits `target` and `activated` | `missing-member`, naming `target` (the lower level) |
| 14 | `target = true` (a Boolean for a `Total`) | `invalid_runtime_input`/`wrong-value-kind`, naming `target` |
| 15 | `earlier = Flag(ready true)` (a record of another type in the model) | `wrong-value-kind`, naming `earlier` |
| 16 | `target = 1001` (outside `Int[0, 1000]`) | `invalid_runtime_input`/`invalid-value`, naming `target` |
| 17 | the slot `Refunds.captures[0]` with `view` a reference to the key `ghost` of `nodes` | `dangling_reference`/`absent-target-in-complete-population`, naming `view` |
| 18 | the slot `Refunds.captures[0]` with `view` a reference to a population of another model | `foreign_reference`/`foreign-universe`, naming `view` |
| 19 | the slot `Refunds.captures[0]` with `view = n5` and no observation | `Incomplete`: `unavailable_observation`/`missing-required-artifact`, naming `view` |
| 20 | the slot `Refunds.captures[0]` with the observation marking `nodes` incomplete | `Incomplete`: `incomplete_population`/`incomplete-scope`, naming `nodes` |
| 21 | the slot `Refunds.captures[0]` with an observation that is a snapshot of another model | FR-106's `invalid_model_binding`/`wrong-model-selection` |
| 22 | the unchanged base call after rows 1 to 21 on the same meter | evaluates: true, and the meter holds only this call's charges |
| 23 | nested fixture, peer descriptor selects otherNodes | evaluates: 5; unlisted compatible otherSpare is irrelevant |
| 24 | row 23 with only the descriptor changed to otherSpare | Incomplete incomplete_population/incomplete-scope at otherSpare's population node |
| 25 | row 24 with otherSpare marked complete but empty | dangling_reference/absent-target-in-complete-population at binder forward with path [Field("peer")], naming otherSpare; o1 in otherNodes does not satisfy it |
| 26 | row 23 with peer descriptor repeated | invalid_runtime_input/duplicate-member at forward, path [Field("peer")], metadata index 1 |
| 27 | row 23 with an added descriptor at scalar total | invalid_runtime_input/unknown-member at forward, path [Field("total")], metadata index 1 |
| 28 | row 23 with the peer descriptor omitted | invalid_runtime_input/missing-member at forward, path [Field("peer")] |
| 29 | row 23 with peer's descriptor naming the nodes population of Node rather than Other | invalid_runtime_input/wrong-role-mapping at forward with the peer path and descriptor index 0 |
| 30 | row 23 with malformed descriptors and a refused observation result from another model | original invalid_model_binding/wrong-model-selection, before metadata validation |
| 31 | a bad slot or duplicate, unknown, missing or wrong-shaped source combined with row 26's descriptors | the existing check-1-through-5 failure, before descriptors or observation admission |
| 32 | row 23 after the failures, on that same meter | evaluates: 5; only this call's evaluation changes the meter |

## Expected Results

Each refusal is one record whose code, cause and locus name the node, member,
index or binder the row changes. A supply is exactly the slot's sources:
duplicates, then keys the expression does not read (the forward binder at a
retry, another template's binder), then missing sources, in that order and in
ascending level. A value is checked against its binder's type, and a reference
against the observation. No refusal charges the meter, so the call that follows
evaluates with the full budget.

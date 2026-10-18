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
exactly the slot's sources, a value that does not fit its binder, and a package
or observation that is not the one the supplier prepared, each with one record
and with nothing charged. Scope: FR-390-AC-3 to FR-390-AC-7.

Catches a crossed node or member that evaluates a neighbour, a missing source
that evaluates as false or zero, a supplied capture that overrides the
evaluator's own, another template's binder read through a shared name, a wrong
or foreign reference that evaluates, and a stale package that evaluates.

Fixture **E** and the observation of TC-1070. The base call is TC-1070 row 8:
the slot `Refund.retry.relation` with the supply `earlier = a`, `later = b`,
`forward = a`, `trigger = off`. Every row below changes the base call as it
states, asserts the one refusal's code, cause and locus, that the evaluator
returned no `Evaluation`, and that the meter's counters are as they were before
the call.

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
| 8 | `prepared_for` the `package_id` of another unit | `stale_dependency`/`content-mismatch` naming both identities |
| 9 | `prepared_for` the package's own `package_id` | evaluates: true, `captures()` `target`, `activated` |
| 10 | `earlier` supplied twice | `invalid_runtime_input`/`duplicate-member`, naming `earlier` |
| 11 | the supply adds `target = a` | `invalid_runtime_input`/`unknown-member`, naming `target` |
| 12 | the supply adds `partialTrigger = off` | `unknown-member`, naming `partialTrigger` |
| 13 | the slot `Refund.trigger.guard` with `forward = a`, `partialTrigger = off` | `unknown-member`, naming `partialTrigger`, though `trigger` is also missing (unknown before missing) |
| 14 | the supply omits `trigger` | `invalid_runtime_input`/`missing-member`, naming `trigger` |
| 15 | the supply omits `forward` and `trigger` | `missing-member`, naming `forward` (the lower level) |
| 16 | `forward = 5` (an integer) | `invalid_runtime_input`/`wrong-value-kind`, naming `forward` |
| 17 | `forward = on` (an object of `plains`, type `Plain`, in the model) | `wrong-value-kind`, naming `forward` |
| 18 | `forward` a reference to a population of another model | `foreign_reference`/`foreign-universe`, naming `forward` |
| 19 | `forward` a reference to the key `ghost` of `nodes` | `dangling_reference`/`absent-target-in-complete-population`, naming `forward` |
| 20 | no observation | `Incomplete`: `unavailable_observation`/`missing-required-artifact`, naming `forward` (the first reference, in ascending level) |
| 21 | the observation marks `nodes` incomplete | `Incomplete`: `incomplete_population`/`incomplete-scope`, naming `nodes` |
| 22 | the observation is a snapshot of another model | FR-106's `invalid_model_binding`/`wrong-model-selection` |
| 23 | the unchanged base call after rows 1 to 22 on the same meter | evaluates: true, and the meter holds only this call's charges |

## Expected Results

Each refusal is one record whose code, cause and locus name the node, member,
index or binder the row changes. A supply is exactly the slot's sources:
duplicates, then unknown keys (a capture, another template's binder), then
missing sources, in that order and in ascending level. A value is checked
against its binder's type, and a reference against the observation. No refusal
charges the meter, so the call that follows evaluates with the full budget.

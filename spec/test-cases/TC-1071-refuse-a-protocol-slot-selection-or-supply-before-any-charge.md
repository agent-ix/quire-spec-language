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
one record and with nothing charged. Scope: FR-390-AC-3 to FR-390-AC-6 and FR-390-AC-68. Rows
23–25 additionally exercise reference admission after emission and strict
contract-reader read-back; they do not evaluate the original in-process
`CheckedPackage`.

Catches a crossed node or member that evaluates a neighbour, a missing source
that evaluates as false or zero, a supplied value the expression does not read
that is ignored, another template's binder read through a shared name, and a
wrong, out-of-domain, foreign or dangling value that evaluates.

For in-process rows 1–21, use fixture **E** and the observation of TC-1070. The
base call for selection and supply rows 1–16 is TC-1070 row 8: slot
`Refund.retry.relation` with `earlier = Attempt(tally 0)`,
`later = Attempt(tally 1)`, `target = 5`, `activated = false`. Each refusal row
changes that call as stated and asserts the refusal's code, cause and locus,
that the evaluator returned no `Evaluation`, and that every meter counter is
unchanged. Rows 17–21 instead exercise `Refunds.captures[0]` with the stated
reference value and observation. Row 22 reruns the unchanged retry base call
after the refusal cases to prove the same meter remains usable.

Reader-backed rows 23–25 are independent end-to-end calls. Emit fixture E as
checked-package-v2, strictly admit the emitted package through the contract
reader, and admit the observation for its selected model `M`. For each row,
select `Refunds.captures[0]` from the reader's retained package/view and supply
`view` with the row's reference value and exact root occurrence metadata
`path = []`, `population = M::nodes`. The rows assert the result or refusal
against the reader-backed call, not against the original in-process package.

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
| 23 | Emit fixture E as checked-package-v2, strictly read it, admit its observation for selected model `M`, then evaluate `Refunds.captures[0]` from the reader's retained package/view with `view = n5`; the root reference path `[]` names `M::nodes` | `Evaluated` value 5. The call uses the reader-backed view and the same admitted observation; it does not fall back to fixture E's in-process `CheckedPackage`. |
| 24 | From the same reader-backed setup, supply `view = ghost`, a reference in `M::Node`'s universe, with root path `[]` naming `M::nodes`; `nodes` is complete and does not contain `ghost` | `Refused`, `dangling_reference`/`absent-target-in-complete-population`, at binder `view`; the record identifies `ghost` and `M::nodes`; no `Evaluation` is returned and all meter counters stay unchanged. |
| 25 | From the same reader-backed setup, supply `view` as a `Node` reference whose universe belongs to a separately admitted foreign model | `Refused`, `foreign_reference`/`foreign-universe`, at binder `view`; the record fields are exactly `required = lowercase-hex(M::Node universe identity)` and `supplied = lowercase-hex(the supplied foreign Node universe identity)`, preserving the cause's actual required/supplied values in that order (FR-390 check 5; FR-100-AC-16); no `Evaluation` is returned and all meter counters stay unchanged. |
| 26 | From the checked-handoff package of recipe **R**, select `Full`'s `recovery.predicate`; then the same predicate with its `sum` bound by a `let`, and with two nested `count`/`sum` queries, with `forall`, `exists`, `filter`, `map` and an unread binder, and a `case` arm binder walked over the terms; supply the three sources, then add a key for the introduced binder; a unit whose query binder is named `targetFull`, as the source is | the sources are `target`, `activated`, `recovered` in each form and never the introduced binder; the extra key refuses `invalid_runtime_input`/`unknown-member` with a zero meter; the checker refuses the shadowing unit as `AmbiguousName` |

## Expected Results

Each refusal is one record whose code, cause and locus name the node, member,
index or binder the row changes. A supply is exactly the slot's sources:
duplicates, then keys the expression does not read (the forward binder at a
retry, another template's binder), then missing sources, in that order and in
ascending level. A value is checked against its binder's type, and a reference
against the observation. No refusal charges the meter, so the call that follows
evaluates with the full budget. In rows 23–25, emission and strict reading
must succeed before the supplied-reference cases begin. The evaluated call
uses the trusted reader's retained package/view, and the observation is FR-106
admitted for that same selected model. Each root-reference occurrence carries
its exact path `[]` and the `M::nodes` declaration; the foreign-universe case
changes only the supplied reference's universe. Its refusal preserves the
required and supplied universe identities, represented as lowercase
hexadecimal strings, in the correspondingly named record fields (FR-100-AC-16;
FR-106's admitted record is retained by FR-390). The positive case proves a
valid reader-backed reference still evaluates; the two negative cases assert
the exact refusal catalog code, cause, binder locus and zero-charge boundary.

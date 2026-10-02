---
id: FR-302
title: "Link dispatch at S3 during unit compile"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-300
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
---
# FR-302: Link dispatch at S3 during unit compile

## Description

When the spine `compile` checks a unit that contains a dispatched call, the
S3 checker SHALL link the dispatch table of the called operation's
redefinition family by [FR-083](FR-083-resolve-unique-most-specific-dispatch.md),
and SHALL produce a checked package only when every called family links. If
any called family fails to link, the compile SHALL refuse at S3 and produce no
checked package.

## Inputs

- A complete-V1 unit, compiled through spine `compile` (ADR-011 S1 to S4),
  with each dispatched call `receiver.member(args)` its invariant,
  precondition and postcondition clauses make (QSpec FR-151 rule
  `quire.model.dispatch.single/v1`).
- The admitted model effective view of each selected domain package.
- The caller's `SpineLimits`, whose model normalization limits carry the
  `family_steps` ceiling FR-083 reads.

## Outputs

A `CheckedPackage` in which each dispatched call's checked node names the
linked dispatch table of its operation, or a `CompileRefusal::Check` holding
every link failure.

## Behavior

### Every called family links at S3

For each distinct operation a dispatched call in the unit names, the checker
SHALL link one dispatch table through `check::checked_dispatch_operation`,
which calls `model::dispatch::link_dispatch` (ADR-016 SC-3). The checked
package SHALL carry each linked table, and S6a SHALL select a call's target
only from the table its checked node names.

### A failed link refuses the compile

If FR-083 linking reports a no-applicable or a multiple-undominated failure
for any subtype of any called family, the compile SHALL refuse at S3 with the
QSpec FR-151 code and cause for that failure (`ambiguous_dispatch`/
`no-applicable` or `ambiguous_dispatch`/`multiple-undominated`), naming the
subtype, the candidates and the dominance pairs. The checker SHALL report
every failing subtype of every called family in one refusal, and SHALL
produce no checked package and no partial dispatch table.

### The family walk is bounded by a caller limit

If a called family's `redefines` walk would exceed the caller's
`family_steps` ceiling, the compile SHALL refuse at S3 with FR-083's
resource-exhaustion cause `family-steps`, naming the limit `family_steps`, its configured value, and the
`SpineLimits` model normalization field that raises it. The ceiling is the
caller's value, with the published default of
`ModelNormalizationLimits::default()`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-302-AC-1 | A unit with dispatched calls on two operations, `size` redefined at `A` and at `B` (`B` a subtype of `A`) and `area` declared at `A`, each called twice, compiles into a checked package carrying exactly one linked dispatch table per called operation; the `size` table maps `A` to `A`'s candidate and `B` to `B`'s, and the checked node of every dispatched call names the table of the operation it calls. | Test (TC-793) |
| FR-302-AC-2 | A unit whose dispatched call's family has two undominated candidates for one concrete subtype refuses at S3 with `ambiguous_dispatch`/`multiple-undominated`, naming the subtype, both candidates and the dominance pairs, and returns no checked package. | Test (TC-794) |
| FR-302-AC-3 | A unit whose dispatched call's family has no applicable candidate for one concrete subtype refuses at S3 with `ambiguous_dispatch`/`no-applicable`, naming the subtype, and returns no checked package. | Test (TC-794) |
| FR-302-AC-4 | A unit with dispatched calls on two operations, each with one failing subtype, refuses once and names both failures. | Test (TC-794) |
| FR-302-AC-5 | A unit whose called family has `n` `redefines` edges compiles with `family_steps = n` and, with `family_steps = n - 1`, refuses at S3 with the resource-exhaustion cause `family-steps` naming `family_steps`, the value `n - 1` and the limits field that raises it. | Test (TC-795) |
| FR-302-AC-6 | A unit whose precondition makes a dispatched call on an operation redefined at `A` and at `B` (`B` a subtype of `A`) compiles; evaluating the precondition with a receiver whose most-specific type is `B` runs `B`'s body, and with a receiver of type `A` runs `A`'s body. | Test (TC-809) |

## Dependencies

- **Upstream:** [FR-083](FR-083-resolve-unique-most-specific-dispatch.md)
  links one family; QSpec FR-151 owns the dispatch rule and its codes;
  ADR-016 SC-3 places dispatch selection at S3;
  [FR-300](FR-300-check-model-forms-through-the-state-model-family.md) types
  the dispatched call.
- **Downstream:** S6a evaluation of a dispatched call reads the linked table;
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) evaluates
  clauses that contain dispatched calls.

## References

- ADR-016 §9 G-7 and §10 OR-1.
- Linear QSL-382 (specification), QSL-68 (implementation).

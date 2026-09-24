---
id: SR-622
title: "Integrity review of the QSL-148 termination ruling and the QSL-229 ResolvedSourcePackage ruling"
type: SpecReview
analysis: integrity
scope: "Changes over origin/main on spec/148-229-rulings: the ADR-012 §3 Value row and §14.1 QSL-148 row; FR-087 Outputs, Behavior, CON-4, AC-7, Dependencies and Status; TC-246; the spec/tests.md TC-246 row and coverage note"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-246
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: references
---

## Summary

The review set is base plus integrity. The diff is small and changes text
only.

Termination (QSL-148). The ADR-012 §14.1 QSL-148 row now states that
termination is `Value`'s whole-package `check::termination::check` pass. It
runs after every declaration is checked and returns at most one refusal per
recursive component. This matches the code: `qsl-semantics/src/check/mod.rs`
calls `termination::check` once over every member, after the per-declaration
loop, and `check/termination.rs` finds the components with Tarjan's
algorithm and refuses once per component. FR-065's Status already says
"Termination is a separate whole-package pass". FR-062 says nothing about
termination. The §3 `Value` row lists termination, and now names the pass.
No contradiction remains.

ResolvedSourcePackage (QSL-229). ADR-011 says what the ruling assumes. The
§6.2 `complete::package` row places import resolution in layer-3 `library`
against I2 import views. §8 maps lane C2 to I2 and `library`. §2.4 places
the profile, definition and model selections at E3, through the QSpec
value-lock accessor and the I1 domain packages. FR-087-AC-7, CON-4, the
Behavior section, the Outputs bullet, TC-246 and the tests.md row now split
the retirement by half, name QSL-6 and QSL-189, and keep
`ResolvedSourcePackage` in place until both successors exist. FR-087 Status
records AC-7 and CON-4 as unbacked.

`quire validate` reports no warnings on the changed files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The first draft of the ADR-012 §3 `Value` row said typing, coercion and range obligations run "through the family's builder". Nothing in the diff's scope establishes that, and §4's builders are for staged clause constructs. Fix: leave the row's list as it was and annotate only termination with its pass. | ADR-012 §3 `Value` row; §4.1 |
| FND-002 | low | The §3 row cited "§14" for the QSL-148 row. The row is in §14.1. | ADR-012 §3, §14.1 |
| FND-003 | medium | The first draft named "E3's identity-preimage builder" as the header-selection half's *owner*, next to QSL-6 as the dependency half's owner. QSL-189 only replaces the `DefinitionLock` constants with the QSpec accessor. No ticket names the builder's resolution of profile, definition and model selections. Fix: call the builder the successor and say the half is "gated on QSL-189". No new ticket is opened, per the ruling. | FR-087 Behavior; TC-246 Description; QSL-189 |
| FND-004 | medium | Model selection has a successor only on a path that admits domain packages at I1. ADR-011 §2.4 says spine `compile` admits none and its E3 refuses a `model` declaration. TC-246 step 4 required every model scenario to have an equivalent without saying where it runs. Fix: FR-087 Behavior states the I1 condition, and TC-246 step 4 runs a model scenario on a path that admits the domain package. | FR-087 Behavior; TC-246 step 4; ADR-011 §2.4 |
| FND-005 | low | ADR-011 §2.4 says the complete-V1 `import … digest …` grammar keys a dependency by `package_id`. Today's `ImportSelection` (`qsl-foundation/src/selection.rs`) carries a `DefinitionRef`, and `resolve_source_package` resolves it against `DefinitionCatalog`. The ruling does not depend on this, and the dependency half's successor is where the two meet. Fix: FR-087 Status records it as a fact. ADR-011 is not edited here. | FR-087 Status; ADR-011 §2.4 |
| FND-006 | low | ADR-011 §6.2's `complete::package` row places the whole of `resolve_source_package` in `library`. It is the current-module map and remains true until the header half moves to E3. No edit. | ADR-011 §6.2 |
| FND-007 | low | Two changed lines ran past the file's wrap width: FR-087 Behavior's lead paragraph and the tests.md coverage note. Fix: reflow. | FR-087 Behavior; spec/tests.md |

## Resolution

- FND-001: the §3 `Value` row keeps "typing, coercion, `Int[..]` range
  obligations, termination" and adds the pass in parentheses.
- FND-002: the row cites §14.1.
- FND-003: FR-087 Behavior and TC-246 say "Gated on QSL-189" for the
  header-selection half.
- FND-004: fixed in FR-087 Behavior and TC-246 step 4.
- FND-005: recorded in FR-087 Status.
- FND-006: no change.
- FND-007: reflowed.

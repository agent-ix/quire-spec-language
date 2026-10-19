---
id: SR-12823
title: "integrity review of STD-169 FR-214 and TC-659 step-row forms"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4ca372951028b143e8da7850d0645f7a34b48269; FR-214-AC-1..AC-3, TC-659; cross-checked against ADR-020, ADR-027 PR-1/§7/§7.1, FR-135, FR-136, FR-212, TC-541 and QSpec shared-grammar.md, FR-375, FR-432, TC-386"
review_set: subset
---

## Summary

Ticket: STD-169 (PR agent-ix/quire-spec-language#977). Cross-artifact
consistency of the rewritten rows. The new rows agree with QSL ADR-020
line 133 (`step <C>::<protocol>::<node> -> …`), ADR-027 PR-1, FR-136's
coverage rules and its own `Impl::Counter::commitA -> Spec::Counter::inc(self, …)`
ACs, TC-541, and QSpec FR-432's left-side table and FR-375's `step-target`.
They parse under PR #974's `StepSource`/`StepTarget` productions and resolve
operation rows the way PR #970 does (`<concrete alias>::<Type>::<op>`). One
low finding concerns how a nested node is addressed.

## Verdict

PASS. Author's open point 1 (`M::Fill::A` versus QSpec TC-386's nested
`Impl::P::a::S`): `M::Fill::A` follows the normative forms, FR-432's table
and ADR-027 PR-1/ADR-020, all `<C>::<protocol>::<node>`. QSpec's
choreography surface resolves a relative node name in lexical scope and gives
a fully qualified path the same identity, so a unique `A` resolves either way.
The inconsistency is upstream. QSpec TC-386 writes branch paths and omits
`<C>` (`Impl::P::a::S`, `Impl::Do`), and PR #974's CST vectors write
`Fill::Both` and `Fill::Main::A` without an alias. Neither FR-432 nor FR-214
says which form a step row uses for a node nested in a sequence, parallel or
branch. FND-001 records this; the fix belongs in QSpec (STD), not in this
PR.

## Examined Scope

```yaml
scope:
  - {id: FR-214-AC-3, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: examined, excerpt: "Adding `step M::Fill::A -> stutter` beside `step M::Cell::inc -> Abs::Acc::add(self)` maps `A`'s steps by the node row (`Internal`) and leaves the `inc` operation row unused by `A`."}
  - {id: FR-214-AC-1, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: examined, excerpt: "rows `step M::Cell::inc -> Abs::Acc::add(self)`, `step M::Cell::setTwo -> Abs::Acc::add(self)`, `step fork M::Fill::Both -> stutter`, `step join M::Fill::Both -> stutter` and `step M::Fill::Done -> stutter`, checks"}
  - {id: FR-214-AC-2, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: examined, excerpt: "Adding a second `step M::Cell::inc -> …` row refuses `invalid_model_binding`/`conflicting-binding`."}
  - {id: TC-659, path: spec/test-cases/TC-659-protocol-refinement-step-rows.md, role: examined, excerpt: "3. Add the node row `step M::Fill::A -> stutter`; check ADR-027 §7.1 without the `cend` row and `Chatter` without the channel row."}
  - {id: ADR-027, path: spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, role: context_only, excerpt: "run sequence Main { parallel Both { branch left attempt A by w on M::Cell::inc ... } join all [left, right]; } finish Done as (d) { true };"}
  - {id: ADR-020, path: spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md, role: context_only, excerpt: "step <C>::<protocol>::<node> -> <A>::<Type>::<op>(…) | stutter | any;   // protocol subject (ADR-027 PR-1)"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-214-AC-3's `step M::Fill::A` addresses an attempt node nested in `Main`/`Both`/`left` by the flat `<C>::<protocol>::<node>` form of FR-432 and ADR-027 PR-1. QSpec TC-386 addresses nested nodes by branch path without `<C>` (`Impl::P::a::S`), and PR #974's CST vectors use `Fill::Main::A`. No normative text says how a step row addresses a nested node, so a resolver built from TC-386 could reject `M::Fill::A`. Route to QSpec (FR-432 table plus TC-386) to state the node segment's form. This PR's choice is the normative one. | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md:78; quire-specification spec/test-cases/TC-386-explicit-protocol-step-rows.md SR-01/SR-02 |
